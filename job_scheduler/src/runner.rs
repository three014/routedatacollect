use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        mpsc::{Receiver, TryRecvError},
        Arc, Condvar, Mutex,
    },
    time::Duration,
};

use futures::future::{BoxFuture, Join};
use tokio::task::JoinHandle;

pub fn runner(
    job_receiver: Receiver<(u32, BoxFuture<'static, crate::Result>)>,
    sleep: Arc<(Mutex<()>, Condvar)>,
) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();

    let mut handles = HashMap::new();
    let mut _leftover_handles = Vec::new(); // Not sure if this will be needed.
    log::info!(target: "runner::runner", "Started.");

    loop {
        match job_receiver.try_recv() {
            Ok((id, future)) => {
                if let Some(replaced_handle) = handles.insert(id, rt.spawn(future)) {
                    _leftover_handles.push((id, replaced_handle));
                }
            }
            Err(TryRecvError::Empty) => {
                if handles.values().all(|handle| handle.is_finished())
                    && _leftover_handles
                        .iter()
                        .all(|(_, handle)| handle.is_finished())
                {
                    drain_handles(&rt,&mut handles, &mut _leftover_handles);
                    log::debug!(target: "runner::runner", "No active jobs running. Going to sleep.");
                    drop(
                        sleep
                            .1
                            .wait_timeout(sleep.0.lock().unwrap(), Duration::from_secs(3600))
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
    drain_handles(&rt,&mut handles, &mut _leftover_handles);
    log::trace!(target: "runner::runner", "Leaving function.");
}

fn drain_handles(rt: &tokio::runtime::Runtime, handles: &mut HashMap<u32, JoinHandle<crate::Result>>, _leftover_handles: &mut Vec<(u32, JoinHandle<crate::Result>)>) {
    rt.block_on(async {
        for (id, handle) in handles.drain().chain(_leftover_handles.drain(..)) {
            match handle.await {
                Ok(result) => if let Err(e) = result {
                    log::warn!(target: "runner::runner", "Job (id={id}) finished with error: {e}");
                },
                Err(e) => log::error!(target: "runner::runner", "Error on awaiting job (id={id}): {e}"),
            }
        }
    });
}
