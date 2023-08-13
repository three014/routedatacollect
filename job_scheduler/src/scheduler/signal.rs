use crate::{
    career::{Job, JobBoard},
    runner::runner,
    utils::{StopSignal, WakeSignal},
};
use chrono::{TimeZone, Utc};
use std::{
    sync::{Arc, Mutex, MutexGuard},
    time::{self, Duration},
};
use tokio::{
    sync::{broadcast, mpsc},
    task::JoinHandle,
};

enum State {
    Sleep(time::Duration),
    Run(Job),
}

impl State {
    const SECONDS_IN_AN_HOUR: u64 = 3600;

    /// Runs the calculation to determine whether any jobs exist,
    /// and when those jobs should be run.
    ///
    /// The function returns `State::Run(career::Job)` if it is time to run
    /// the next job, or `State::Sleep(time::Duration)` so that the
    /// clock can sleep until it is time to run the next job.
    ///
    /// If the function determined it was time to run the next job, but
    /// couldn't due to the job completing or expiring, then the
    /// function returns `None`. Futhermore, that is the only reason
    /// why this function can return `None`.
    pub fn determine<T>(jobs: &mut MutexGuard<JobBoard<T>>) -> Option<Self>
    where
        T: TimeZone + Send + Sync + 'static,
        T::Offset: Send,
    {
        if let Some(exec_time) = jobs.peek_next() {
            let now = Utc::now();
            let then = exec_time.with_timezone(&Utc);
            if then > now {
                let diff = then - now;
                Some(State::Sleep(diff.to_std().unwrap_or_default()))
            } else {
                jobs.try_run_next().map(State::Run)
            }
        } else {
            Some(State::Sleep(Duration::from_secs(Self::SECONDS_IN_AN_HOUR)))
        }
    }
}

#[derive(Debug)]
enum Stoplight {
    Running {
        handle: Option<JoinHandle<()>>,
        stopper: Option<broadcast::Sender<StopSignal>>,
        waker: mpsc::Sender<WakeSignal>,
    },
    Pausing,
    Starting,
    Stopped,
}

impl Stoplight {
    pub fn active(&self) -> bool {
        matches!(self, Stoplight::Stopped)
    }
}

#[derive(Debug)]
pub struct Signal {
    light: Mutex<Stoplight>,
}

impl Signal {
    const PADDING: u64 = 200;
    const STOP: StopSignal = StopSignal;
    const WAKE_UP: WakeSignal = WakeSignal;

    pub fn new() -> Self {
        Self {
            light: Mutex::new(Stoplight::Stopped),
        }
    }

    pub async fn start<T>(&mut self, jobs: Arc<Mutex<JobBoard<T>>>)
    where
        T: TimeZone + Send + Sync + 'static,
        T::Offset: Send,
    {
        let mut lock = self.light.lock().expect("weird");
        if let Stoplight::Stopped = &*lock {
            *lock = Stoplight::Starting;
            drop(lock);
            log::info!("Starting scheduler service");

            let (stopper, stop_recv) = broadcast::channel::<StopSignal>(15);
            let (waker, wake_recv) = mpsc::channel::<WakeSignal>(15);
            let handle = tokio::spawn(Signal::clock(jobs, wake_recv, stop_recv));

            let mut lock = self
                .light
                .lock()
                .expect("should be no problems after spawning clock");
            *lock = Stoplight::Running {
                handle: Some(handle),
                stopper: Some(stopper),
                waker,
            }
        }
    }

    async fn clock<T>(
        jobs: Arc<Mutex<JobBoard<T>>>,
        mut wake_recv: mpsc::Receiver<WakeSignal>,
        mut stop_recv: broadcast::Receiver<StopSignal>,
    ) where
        T: TimeZone + Send + Sync + 'static,
        T::Offset: Send,
    {
        log::trace!("Initializing clock.");
        let (job_send, job_recv) = mpsc::channel::<Job>(100);
        let runner_stop_recv = stop_recv.resubscribe();
        let handle = tokio::spawn(runner(job_recv, runner_stop_recv));
        log::trace!("Started runner, ready to drive jobs!");

        while let Err(broadcast::error::TryRecvError::Empty) = stop_recv.try_recv() {
            let state = {
                let mut jobs = jobs.lock().unwrap();
                Self::determine_state(&mut jobs)
            };

            match state {
                State::Sleep(duration) => {
                    log::debug!("About to sleep for {:?}.", &duration);
                    tokio::select! {
                        biased;
                        _ = stop_recv.recv() => break,
                        _ = wake_recv.recv() => (),
                        _ = tokio::time::sleep(duration + padding()) => ()
                    }
                }
                State::Run(job) => {
                    log::info!("Running job with id: {}!", job.id);
                    if let Err(err) = job_send.send_timeout(job, padding()).await {
                        // Error logging here
                        log::error!("Failed to send job to runner: {err}");
                    }
                }
            }
        }

        // Cleanup
        log::trace!("Stopping runner.");
        drop(job_send);
        if let Err(err) = handle.await {
            // Error logging here
            log::error!("Unable to join runner: {err}.");
        }
    }

    /// Calls on `State::determine`, but loops until the
    /// function doesn't return a `None` value.
    ///
    /// In the current implentation, `State::determine` only
    /// returns `None` when the selected job was expired,
    /// but when that happens, the expired job gets removed
    /// from the event queue.
    /// In that case, it makes sense to call function again
    /// until it returns a `State::Sleep` or `State::Job`,
    /// since we can use it to remove expired jobs.
    fn determine_state<T>(jobs: &mut MutexGuard<JobBoard<T>>) -> State
    where
        T: TimeZone + Send + Sync + 'static,
        T::Offset: Send,
    {
        loop {
            if let Some(state) = State::determine(jobs) {
                break state;
            }
        }
    }

    /// Signals the internal clock and runner to stop, if and only if they
    /// are in the running state. For each call to this function, however,
    /// there is a call to unlock the internal mutex, so the cost is not
    /// necessarily zero if the internals aren't running.
    pub async fn stop(&mut self) {
        let mut lock = self.light.lock().expect("should've been able to lock");
        if let Stoplight::Running {
            handle,
            stopper,
            waker: _,
        } = &mut *lock
        {
            let handle = handle
                .take()
                .expect("there should be a handle if stoplight is running");
            let stopper = stopper
                .take()
                .expect("there should be a sender if stoplight is running");
            *lock = Stoplight::Pausing;
            drop(lock);

            log::trace!("Sending stop signal to clock.");
            if let Err(err) = stopper.send(Signal::STOP) {
                // Error logging here
                log::error!("Unable to send stop signal to clock: {err}");

                handle.abort()
            } else if let Err(err) = handle.await {
                // Error logging here too
                log::error!("Unable to join on clock: {err}");
            }

            let mut lock = self.light.lock().expect("should be able to lock");
            *lock = Stoplight::Stopped;
        }
    }

    /// Returns whether the signal's internal clock is running.
    /// The current implementation says that the clock is considered
    /// active if it is in any of the starting, running, or pausing states, but not when it's
    /// in the stopped state.
    pub fn active(&self) -> bool {
        let lock = self.light.lock().unwrap();
        lock.active()
    }

    /// Sends a wake signal to the internal clock, if it is running.
    /// In the case that the send fails, the resulting timeout error
    /// is logged.
    ///
    /// Should be used when new jobs are added to the scheduler, in
    /// order to ensure that the clock recalulates the amount of time
    /// it needs to sleep before the it runs the next job.
    pub async fn wake(&self) {
        let lock = self.light.lock().unwrap();
        if let Stoplight::Running {
            handle: _,
            stopper: _,
            waker,
        } = &*lock
        {
            let wake = waker.send_timeout(Signal::WAKE_UP, padding());
            if let Err(_err) = wake.await {
                // Error logging here
            }
        } 
    }
}

/// Helper function that returns a `std::time::Duration`
/// equal to `Signal::PADDING` in milliseconds.
fn padding() -> Duration {
    Duration::from_millis(Signal::PADDING)
}
