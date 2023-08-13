use self::signal::Signal;
use crate::{
    career::{error::DescheduleError, JobBoard},
    AsyncFn, Limit,
};
use chrono::{TimeZone, Utc};
use std::sync::{Arc, Mutex};

mod signal;

pub struct Scheduler<T>
where
    T: TimeZone + Send + Sync + 'static,
    T::Offset: Send,
{
    timezone: T,
    job_board: Arc<Mutex<JobBoard<T>>>,
    signal: Signal,
}

impl Scheduler<Utc> {

    /// Returns a new `Scheduler` with the `Utc` timezone.
    pub fn new() -> Self {
        Self::with_timezone(Utc)
    }
}

impl Default for Scheduler<Utc> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Scheduler<T>
where
    T: TimeZone + Send + Sync + 'static,
    T::Offset: Send,
{
    /// Returns a new `Scheduler` with the supplied
    /// timezone. All datetimes within this scheduler
    /// will be assumed to be of that timezone.
    pub fn with_timezone(timezone: T) -> Self {
        Self {
            timezone: timezone.clone(),
            job_board: Arc::new(Mutex::new(JobBoard::new(timezone))),
            signal: Signal::new(),
        }
    }

    pub async fn start(&mut self) {
        let job_board = self.job_board.clone();
        self.signal.start(job_board).await;
        log::info!("Started.");
    }

    pub async fn stop(&mut self) {
        self.signal.stop().await;
        log::info!("Stopped.");
    }

    pub async fn shutdown(&mut self) {
        self.stop().await;
        self.job_board
            .lock()
            .expect("no one should be accessing job_board at this point")
            .clear();
        log::info!("Shutdown complete.");
    }

    pub async fn restart(&mut self) {
        log::info!("Restarting.");
        self.stop().await;
        self.start().await
    }

    pub fn active(&self) -> bool {
        self.signal.active()
    }

    /// Schedules a new job to run on the service. Refer to `Scheduler` for more
    /// information on what a `Job` function looks like.
    ///
    /// Along with the function, the user must also supply a `cron::Schedule`,
    /// which can be parsed from a string using the `parse` method, and
    /// a `job_scheduler::Limit` if the user desires to automatically stop
    /// scheduling this job at some point later-on.
    ///
    /// The function returns a `Result` containing the job id on success
    /// and a string on failure containing the reason for failing to add the
    /// job to the internal queue.
    pub async fn schedule<J>(
        &mut self,
        schedule: cron::Schedule,
        limit_num_execs: Option<Limit>,
        job: J,
    ) -> Result<usize, &'static str>
    where
        J: AsyncFn + Clone + Send + 'static,
    {
        let result = self
            .job_board
            .lock()
            .map(|mut jobs| {
                jobs.schedule(self.timezone.clone(), schedule, limit_num_execs, job)
            })
            .map_err(|err| {
                // Log error here
                log::error!("{err}. Shutting down.");
                "Inner lock poisoned. Scheduler shutdown."
            });

        if result.is_err() {
            self.shutdown().await;
        } else {
            self.signal.wake().await;
        }
        result
    }

    pub async fn deschedule(&mut self, job_id: usize) -> Result<(), DescheduleError> {
        self.job_board
            .lock()
            .map(|mut jobs| jobs.deschedule(job_id))
            .map_err(|err| DescheduleError::LockPoisioned(err.to_string()))?
    }
}