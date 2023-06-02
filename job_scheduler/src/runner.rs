use self::utils::Inner;
use crate::JobId;
use futures::future::BoxFuture;
use fxhash::FxHasher32;
use std::{
    collections::{hash_map, HashMap, VecDeque},
    hash::BuildHasherDefault,
    sync::{
        mpsc::{Receiver, TryRecvError},
        Arc, Condvar, Mutex,
    },
    time::Duration,
};
use tokio::task::JoinHandle;

pub fn runner(
    job_receiver: Receiver<(JobId, BoxFuture<'static, crate::Result>)>,
    runner_go_sleep: Arc<(Mutex<()>, Condvar)>,
    running_jobs_report: Arc<Mutex<RunningJobs>>,
) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    let (sender, receiver) = std::sync::mpsc::channel::<JobId>();
    let mut tokio_receiver = crate::receiver::PeekableReciever::from_receiver(receiver);
    log::info!(target: "runner::runner", "Started.");

    loop {
        let waiting_on_jobs;
        if let Ok(id) = tokio_receiver.try_recv() {
            if let Ok(mut handles) = running_jobs_report.lock() {
                if let hash_map::Entry::Occupied(mut queue) = handles.inner.entry(id) {
                    if let Some(handle) = queue.get_mut().pop_front() {
                        rt.block_on(handle_result(id, handle));
                    } else {
                        queue.remove_entry();
                    }
                }
            }
            waiting_on_jobs = tokio_receiver.peek().is_ok();
        } else {
            waiting_on_jobs = false;
        }

        // Check if there are any jobs to process, and/or if the runner should sleep.
        match job_receiver.try_recv() {
            Ok((id, future)) => {
                let alert_runner = sender.clone();
                let runner_wake_up = runner_go_sleep.clone();
                let handle = rt.spawn(async move {
                    // Run the job.
                    let result = future.await;
                    // Tell the runner we're finished.
                    alert_runner.send(id).expect("Receiver pipe should be open, since it's on the runner thread. Runner thread shouldn't crash (easily).");
                    // Wake up the runner just in case.
                    runner_wake_up.1.notify_all();
                    // Return the result.
                    result
                });
                if let Ok(mut handles) = running_jobs_report.lock() {
                    handles
                        .inner
                        .entry(id)
                        .or_insert_with(VecDeque::new)
                        .push_back(handle);
                } else {
                    log::error!(target: "runner::runner", "Couldn't add running job to report. Might be a runaway.");
                }
            }
            Err(TryRecvError::Empty) => {
                // Try to allow the thread to endlessly loop when nothing else to do.
                // If there's no more jobs to start, then the runner should:
                // - Check if there are finished jobs
                //   - Lock the report, collect the result of that job only
                // This will probably still be costly due to the repeated acquiring of the
                // lockguard for the report, __unless there are no more jobs__, to which
                // the thread should effectly check two-ish conditionals and do nothing.
                //
                // Will this work??
                // Update: Maybe not???
                // Update: Going back to allowing sleeping, but only sleeping, no housekeeping.
                if !waiting_on_jobs {
                    log::debug!("No new jobs and no finished jobs. Going to sleep.");
                    drop(
                        runner_go_sleep
                            .1
                            .wait_timeout(
                                runner_go_sleep.0.lock().unwrap(),
                                Duration::from_secs(600),
                            )
                            .unwrap()
                            .0,
                    )
                }
                /*
                let mut lock = running_jobs_report.try_lock().unwrap();
                if lock.inner.values().all(|handle| handle.is_finished()) && !waiting_on_jobs {
                    drain_handles(&rt, &mut lock.inner); // Housekeeping
                    drop(lock);
                    log::debug!(target: "runner::runner", "No active jobs running. Going to sleep.");
                    drop(
                        runner_go_sleep
                            .1
                            .wait_timeout(
                                runner_go_sleep.0.lock().unwrap(),
                                Duration::from_secs(3600),
                            )
                            .unwrap()
                            .0,
                    );
                }
                */
            }
            Err(TryRecvError::Disconnected) => {
                break;
            }
        }
    }

    log::info!(target: "runner::runner", "Reciever disconnected, waiting for current jobs to finish.");
    shutdown(&rt, &mut running_jobs_report.lock().unwrap().inner);
    rt.shutdown_timeout(Duration::from_millis(100));
    log::trace!(target: "runner::runner", "Leaving function.");
}

// fn drain_handles(
//     rt: &tokio::runtime::Runtime,
//     handles: &mut HashMap<
//         JobId,
//         VecDeque<JoinHandle<crate::Result>>,
//         BuildHasherDefault<FxHasher32>,
//     >,
// ) {
//     rt.block_on(async {
//         for (id, task) in handles.drain() {
//             handle_result(id, task).await;
//         }
//     });
// }

fn shutdown(
    rt: &tokio::runtime::Runtime,
    handles: &mut HashMap<
        JobId,
        VecDeque<JoinHandle<crate::Result>>,
        BuildHasherDefault<FxHasher32>,
    >,
) {
    rt.block_on(async {
        tokio::select! {
            _ = async {
                for (id, mut task) in handles.drain() {
                    for handle in task.drain(..) {
                        handle_result(id, handle).await;
                    }
                }
            } => {},
            _ = tokio::time::sleep(Duration::from_secs(5)) => {}
        }
    });
}

async fn handle_result(id: JobId, handle: JoinHandle<crate::Result>) {
    match handle.await {
        Ok(result) => {
            if let Err(e) = result {
                log::warn!(target: "runner::handle_result", "Job (id={id}) finished with error: {e:?}");
            } else {
                log::info!(target: "runner::handle_result", "Job (id={id}) finished normally.");
            }
        }
        Err(e) => {
            log::error!(target: "runner::handle_result", "Error on awaiting job (id={id}): {e:?}")
        }
    }
}

pub struct RunningJobs {
    inner: Inner,
}

impl RunningJobs {
    pub fn contains(&self, id: &JobId) -> bool {
        self.inner.contains_key(id)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        RunningJobs {
            inner: utils::Inner {
                map: HashMap::with_capacity_and_hasher(capacity, BuildHasherDefault::default()),
            },
        }
    }
}

mod utils {
    use crate::JobId;
    use fxhash::FxHasher32;
    use std::{
        collections::{HashMap, VecDeque},
        hash::BuildHasherDefault,
        ops::{Deref, DerefMut},
    };
    use tokio::task::JoinHandle;

    pub struct Inner {
        pub map:
            HashMap<JobId, VecDeque<JoinHandle<crate::Result>>, BuildHasherDefault<FxHasher32>>,
    }

    impl Deref for Inner {
        type Target =
            HashMap<JobId, VecDeque<JoinHandle<crate::Result>>, BuildHasherDefault<FxHasher32>>;

        fn deref(&self) -> &Self::Target {
            &self.map
        }
    }

    impl DerefMut for Inner {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.map
        }
    }
}
