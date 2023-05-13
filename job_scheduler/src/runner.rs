use std::{
    collections::VecDeque,
    sync::{
        mpsc::{Receiver, TryRecvError},
        Arc, Condvar, Mutex,
    },
    time::Duration,
};

use futures::future::BoxFuture;

use crate::job::JobResult;

pub fn runner(
    job_receiver: Receiver<BoxFuture<'static, JobResult>>,
    sleep: Arc<(Mutex<()>, Condvar)>,
) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();


    let mut handles = VecDeque::new();
    log::info!(target: "runner", "Started.");

    loop {
        match job_receiver.try_recv() {
            Ok(future) => handles.push_back(rt.spawn(future)),
            Err(TryRecvError::Empty) => {
                if handles.iter().all(|handle| handle.is_finished()) {
                    log::debug!(target: "runner", "No active jobs running. Going to sleep.");
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

    log::info!(target: "runner", "Reciever disconnected, waiting for current jobs to finish.");
    rt.block_on(async {
        futures::future::join_all(handles.into_iter()).await
    });
    log::trace!(target: "runner", "Leaving function.");
}
