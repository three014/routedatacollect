use std::{
    collections::HashMap,
    sync::{
        mpsc::{Receiver, TryRecvError},
        Arc, Condvar, Mutex,
    },
    time::Duration,
};

use futures::future::BoxFuture;
use tokio::task::JoinHandle;

pub fn runner(
    job_receiver: Receiver<(u32, BoxFuture<'static, crate::Result>)>,
    runner_go_sleep: Arc<(Mutex<()>, Condvar)>,
    starting_num_jobs: usize,
) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();

    let (sender, receiver) = std::sync::mpsc::channel::<u32>();
    let mut tokio_receiver = crate::receiver::PeekableReciever::from_receiver(receiver);
    let mut handles = HashMap::with_capacity(starting_num_jobs);
    log::info!(target: "runner::runner", "Started.");

    loop {
        // Check if there are any handles to join.
        let waiting_on_jobs;
        if let Ok(id) = tokio_receiver.try_recv() {
            if let Some(job_handle) = handles.remove(&id) {
                rt.block_on(async {
                    // Should be a super small block, since the task should be done.
                    handle_result(id, job_handle).await;
                });
            }
            waiting_on_jobs = tokio_receiver.peek().is_some();
        } else {
            waiting_on_jobs = false;
        }

        // Check if there are any jobs to process, and/or if the runner should sleep.
        match job_receiver.try_recv() {
            Ok((id, future)) => {
                let alert_runner = sender.clone();
                let runner_wake_up = runner_go_sleep.clone();
                let check = handles.insert(id, rt.spawn(async move {
                    // Run the job.
                    let result = future.await;
                    // Tell the runner we're finished.
                    alert_runner.send(id).expect("Receiver pipe should be open, since it's on the runner thread. Runner thread shouldn't crash (easily).");
                    // Wake up the runner just in case.
                    runner_wake_up.1.notify_one();
                    // Return the result.
                    result
                }));
                assert!(
                    check.is_none(),
                    "Replaced job that shouldn't have been replaced."
                );
            }
            Err(TryRecvError::Empty) => {
                if handles.values().all(|handle| handle.is_finished()) && !waiting_on_jobs {
                    drain_handles(&rt, &mut handles); // Housekeeping
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
            }
            Err(TryRecvError::Disconnected) => {
                break;
            }
        }
    }

    log::info!(target: "runner::runner", "Reciever disconnected, waiting for current jobs to finish.");
    drain_handles(&rt, &mut handles);
    log::trace!(target: "runner::runner", "Leaving function.");
}

fn drain_handles(
    rt: &tokio::runtime::Runtime,
    handles: &mut HashMap<u32, JoinHandle<crate::Result>>,
) {
    rt.block_on(async {
        for (id, task) in handles.drain() {
            handle_result(id, task).await;
        }
    });
}

async fn handle_result(id: u32, handle: JoinHandle<crate::Result>) {
    match handle.await {
        Ok(result) => {
            if let Err(e) = result {
                log::warn!(target: "runner::handle_result", "Job (id={id}) finished with error: {e}");
            } else {
                log::info!(target: "runner::handle_result", "Job (id={id}) finished normally.");
            }
        }
        Err(e) => {
            log::error!(target: "runner::handle_result", "Error on awaiting job (id={id}): {e}")
        }
    }
}
