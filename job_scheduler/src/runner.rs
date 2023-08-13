use crate::{
    career::Job,
    utils::{map::SimpleMap, StopSignal},
};
use tokio::{
    sync::{broadcast, mpsc},
    task::JoinHandle,
};

const DEFAULT_CAPACITY: usize = 4;

pub(crate) async fn runner(
    mut job_recv: mpsc::Receiver<Job>,
    mut stop_recv: broadcast::Receiver<StopSignal>,
) {
    log::trace!("Initializing runner.");
    // We create a map with a nonzero capacity so the linear scan actually attempts to find open keys.
    let mut active_jobs: SimpleMap<SimpleMap<JoinHandle<crate::Result>>> =
        SimpleMap::with_capacity(DEFAULT_CAPACITY);
    let (finished_send, mut finished_recv) = mpsc::channel::<(usize, usize)>(100);

    loop {
        tokio::select! {
            biased;
            _ = stop_recv.recv() => break,
            result = finished_recv.recv() => {
                log::debug!("Received a finished job.");
                if let Some((id, pos)) = result {
                    let map = active_jobs.get_mut(id).expect("should have id in map if task is active");
                    let mut handle = map.remove(pos).expect("should have a pos if task is active");
                    handle_result(id, &mut handle).await;
                }
            }
            result = job_recv.recv() => {
                log::debug!("Received a new job to run!");
                if let Some(Job { id, command }) = result {
                    let notifier = finished_send.clone();
                    let id_map = active_jobs
                        .entry(id)
                        .or_insert_with(|| SimpleMap::with_capacity(DEFAULT_CAPACITY));
                    for pos in 0..id_map.capacity() {
                        if !id_map.contains_key(pos) {
                            let handle = tokio::spawn(async move {
                                // Run the job
                                let result = command.await;
                                // Tell the runner we're finished
                                notifier.send((id, pos)).await.expect("Recv pipe should be open");
                                // Return the result
                                result
                            });
                            id_map.insert(pos, handle);
                            break;
                        }
                    }
                }
            }
        }
    }

    // clean-up
    for (id, map) in active_jobs.iter_mut() {
        for handle in map.values_mut() {
            if handle.is_finished() {
                handle_result(id, handle).await;
            } else {
                handle.abort();
            }
        }
    }
}

async fn handle_result(id: usize, handle: &mut JoinHandle<crate::Result>) {
    match handle.await {
        Ok(result) => {
            if let Err(err) = result {
                log::warn!("Job with id: {id} finished with error: {err:?}");
            } else {
                log::info!("Job with id: {id} finished normally.");
            }
        }
        Err(err) => {
            log::error!("Error on awaiting job with id: {id}: {err:?}")
        }
    }
}
