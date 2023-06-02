use crate::{job, runner, AsyncFn, JobId};
use chrono::{TimeZone, Utc};
use futures::future::BoxFuture;
use std::{
    sync::{mpsc, Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

/// The Scheduling System itself. Scheduling
/// a new job is done through the `add_job` method,
/// but the internal clock and subsequently all
/// added jobs will not run until `start` is called.
///
/// All job schedules follow the Linux Cron syntax,
/// and the date-intervals are interpreted with
/// whatever timezone that was used to create the
/// scheduler.
/// 
/// A `Job` is an `impl FnOnce + Clone + Send + 'static` function pointer or closure
/// that returns an `impl Future + Send + 'static` with the return type of
/// `Result<(), Box<dyn Error + Send + Sync>>`. This means that a job
/// can be used to mutate a shared state, so long as all items in the job
/// implement `Clone` and `Send`.
///
/// # Examples
///
/// ```
/// use job_scheduler::{Limit, Scheduler};
///
/// let mut s = Scheduler::with_timezone(chrono_tz::America::Chicago);
/// s.add_job(
///     || async {
///         println!("Hello World!");
///         Ok(())
///     },
///     "00 * * * * *".parse().unwrap(),
///     Limit::NumTimes(5),
/// );
///
/// s.start();
/// // "Hello World!" will have printed 5 times in this duration.
/// std::thread::sleep(std::time::Duration::from_secs(360));
/// s.stop();
/// ```
///
/// Using Shared-State:
///
/// ```
/// use job_scheduler::{Limit, Scheduler};
/// use std::sync::{Arc, Mutex};
///
/// let mut s = Scheduler::with_timezone(chrono_tz::America::Chicago);
/// let shared = Arc::new(Mutex::new(0));
/// let shared_copy = shared.clone();
///
/// s.add_job(
///     move || async move {
///         let mut num = shared_copy.lock().unwrap();
///         *num += 1;
///         Ok(())
///     },
///     "00 * * * * *".parse().unwrap(),
///     Limit::NumTimes(3),
/// );
///
/// s.start();
/// std::thread::sleep(std::time::Duration::from_secs(240));
/// s.stop();
///
/// assert_eq!(*shared.lock().unwrap(), 3);
/// ```
///
/// The scheduler implements the `Drop` trait so
/// that `stop` is automatically called when the
/// scheduler object is dropped, although you can
/// call `stop` at any time.
///
/// `Stop` will kill all currently running jobs,
/// but won't clear out the schedules and added
/// jobs. Therefore, you can call `start` to resume
/// the service, and all jobs will run at their next
/// availiable time.
///
/// Overall, this scheduler works best when your jobs
/// are very IO-bound, like making tons of RPC calls
/// to Google Routes API, for example.
///
/// # Failures
///
/// Jobs that panic do not stop the scheduler. Instead,
/// they are collected immediately and a `WARN` log
/// is printed to stderr if logging is enabled.
///
/// If the thread that holds the async runtime itself
/// panics, then the scheduler won't find out until
/// the internal clock tries to run a new job, which
/// then will result in the scheduler calling `stop` on
/// itself and printing an `ERROR` log to stderr if
/// logging is enabled.
///
/// If the internal clock panics, the thread that
/// holds the async runtime will stop as soon as it
/// checks for a new job to run, which can occur
/// instantly or within an hour.
///
/// It is up to the user of the scheduler to
/// check if the scheduler has crashed and decide
/// what to do afterwards, but in future
/// I plan to add more options to handle crashes.
pub struct Scheduler<T>
where
    T: TimeZone + Copy + Clone + Send + Sync + 'static,
    T::Offset: Send,
{
    clock: Option<JoinHandle<()>>,
    service_running: Arc<(Mutex<bool>, Condvar)>,
    timezone: T,
    job_board: Arc<Mutex<job::JobBoard<T>>>,
}

impl Scheduler<Utc> {
    /// Returns a new `Scheduler` with the `Utc` timezone.
    pub fn new() -> Self {
        Self {
            clock: None,
            timezone: Utc,
            service_running: Arc::new((Mutex::new(false), Condvar::new())),
            job_board: Arc::new(Mutex::new(job::JobBoard::new(Utc))),
        }
    }
}

impl Default for Scheduler<Utc> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Scheduler<T>
where
    T: TimeZone + Copy + Clone + Send + Sync + 'static,
    T::Offset: Send,
{
    const SECONDS_IN_AN_HOUR: u64 = 3600;
    const PADDING: u64 = 200;

    /// Returns a new `Scheduler` with the supplied
    /// timezone. All datetimes within this scheduler
    /// will be assumed to be of that timezone.
    pub fn with_timezone(timezone: T) -> Self {
        Self {
            clock: None,
            timezone,
            service_running: Arc::new((Mutex::new(false), Condvar::new())),
            job_board: Arc::new(Mutex::new(job::JobBoard::new(timezone))),
        }
    }

    /// Starts the scheduling service, which consists
    /// of the internal clock, which determines the soonest job
    /// to run and creates the `Future` from that job, and the "runner",
    /// which holds the async runtime and subsequently polls the futures
    /// given to it.
    ///
    /// `start` can be called multiple times, but only does anything
    /// if the service is not already active.
    pub fn start(&mut self) {
        if self.active() {
            return;
        } // DO NOT START NEW THREAD IF ALREADY ACTIVE

        let running = self.service_running.clone();
        let jobs = self.job_board.clone();
        let lock = self.job_board.lock().unwrap();
        let running_jobs_report = lock.currently_running();
        drop(lock);

        // START
        *self.service_running.0.lock().unwrap() = true;
        log::info!(target: "scheduler::Scheduler::start", "Starting service.");

        self.clock = Some(thread::spawn(move || {
            enum ClockState {
                Sleep(Duration),
                Run((JobId, BoxFuture<'static, crate::Result>)),
                Pass,
            }

            // Create a new thread, channel, and condition variable.
            // New thread gets receiving channel, curr thread gets sender channel.
            // Both threads get a copy of the condition variable so that the
            // clock can tell the runner to wake up when necessary.
            let (sender, reciever) = mpsc::channel::<(JobId, BoxFuture<'static, crate::Result>)>();
            let sleep = Arc::new((Mutex::new(()), Condvar::new()));
            let sleep_for_runner = sleep.clone();
            let runner_handle = thread::spawn(move || {
                runner::runner(reciever, sleep_for_runner, running_jobs_report);
            });

            while {
                match running.0.lock() {
                    Ok(guard) => *guard,
                    Err(_) => false, // If main thread panicked, stop service.
                }
            } {
                let mut state = ClockState::Pass;
                let mut jobs = jobs.lock().unwrap();

                if let Some(exec_time) = jobs.peek_next() {
                    let now = Utc::now();
                    let then = exec_time.with_timezone(&Utc);
                    log::debug!("now: {:?}, then: {:?}", &now, &then);
                    if then > now {
                        log::debug!(target: "scheduler::process_manager_thread", "Can't run yet, time is in the future: {:?}.", then);
                        let diff = then - now;

                        state = ClockState::Sleep(diff.to_std().unwrap_or(Duration::from_secs(0)));
                    } else {
                        log::debug!(target: "scheduler::process_manager_thread", "Attempting to exec job.");
                        if let Ok(job) = jobs.try_run_next() {
                            state = ClockState::Run(job);
                        } else {
                            log::debug!(target: "scheduler::process_manager_thread", "Couldn't run job.");
                        }
                    }
                } else {
                    state = ClockState::Sleep(Duration::from_secs(Self::SECONDS_IN_AN_HOUR));
                }

                drop(jobs);

                match state {
                    ClockState::Sleep(duration) => {
                        log::debug!(target: "scheduler::process_manager_thread", "About to sleep for {:?}.", &duration);
                        drop(
                            running
                                .1
                                .wait_timeout(
                                    running.0.lock().unwrap(),
                                    duration + Duration::from_millis(Self::PADDING),
                                )
                                .unwrap()
                                .0,
                        );
                        // Wait a little bit after being woken up so main can set `running` if needed.
                        //thread::sleep(Duration::from_millis(Self::PADDING));
                    }
                    ClockState::Run(job) => {
                        log::info!(target: "scheduler::process_manager_thread", "Running job (id={})!", job.0);
                        if let Err(e) = sender.send(job) {
                            log::error!(target: "scheduler::process_manager_thread", "{e}. Attempting to stop process manager.");
                            *running.0.lock().unwrap() = false; // Stop loop
                        } else {
                            sleep.1.notify_all();
                        }
                    }
                    ClockState::Pass => (),
                }
            }
            log::trace!(target: "scheduler::process_manager_thread", "Ending process manager thread.");

            // Cleanup | TODO: Error handling please
            drop(sender);
            sleep.1.notify_one();
            if let Err(e) = runner_handle.join() {
                log::error!(target: "scheduler::process_manager_thread", "{:?}", e);
            }
            log::trace!(target: "scheduler::process_manager_thread", "Leaving closure.");
        }));
    }

    /// Stops the scheduling service, giving all currently running
    /// jobs about 5 seconds to finish before terminating them automatically.
    ///
    /// Does not remove jobs from the internal queue, so calling `start` will
    /// resume the internal clock and pick/run the next available job.
    ///
    /// `stop` can be called multiple times, but only does anything if
    /// the service is active.
    ///
    /// The scheduler will automatically call `stop` when the scheduler itself
    /// is dropped, due to the custom `Drop` implementation.
    pub fn stop(&mut self) {
        if !self.active() {
            return;
        }
        log::info!(target: "scheduler::Scheduler::stop", "Stopping service, waiting for all processes to finish.");
        *self.service_running.0.lock().unwrap() = false;
        self.service_running.1.notify_one();
        if let Some(handle) = self.clock.take() {
            if let Err(e) = handle.join() {
                log::error!(target: "scheduler::Scheduler::stop", "Unable to join process manager thread during shutdown: {:?}", e);
            }
        }
        log::info!(target: "scheduler::Scheduler::stop", "Stopped.");
    }

    /// Stops and starts the scheduling service, following the rules of 
    /// `stop` and `start`, in that order.
    pub fn restart(&mut self) {
        log::info!(target: "schedule::Scheduler::restart", "Restarting service.");
        self.stop();
        self.start();
    }

    /// Returns whether the service is actively running.
    pub fn active(&self) -> bool {
        self.clock.is_some() && *self.service_running.0.lock().unwrap()
    }

    /// Adds a new job to the scheduler. Refer to [`Scheduler`] for more
    /// information on what a `Job` function looks like. 
    /// 
    /// Along with the function, the user must also supply a `cron::Schedule`,
    /// which can be parsed from a string using the `parse` method, and 
    /// a `job_scheduler::Limit` if the user desires to automatically stop
    /// scheduling this job at some point later-on.
    /// 
    /// `add_job` cannot panic nor fail, but in the case that the service had
    /// crashed, the scheduler will still add the job to the queue.
    /// 
    /// Returns a `JobId` which is just a `u32` for the job just submitted.
    /// This can be used later to remove the job manually if desired.
    pub fn add_job<C>(
        &mut self,
        command: C,
        schedule: cron::Schedule,
        limit_num_execs: crate::Limit,
    ) -> JobId
    where
        C: AsyncFn + Send + 'static,
    {
        let (job_id, should_stop_service) = match self.job_board.lock() {
            Ok(mut jobs) => (
                jobs.schedule_with_limit(command, schedule, self.timezone, limit_num_execs),
                false,
            ),
            Err(mut e) => {
                log::error!(target: "scheduler::Scheduler::add_job", "{e}. Service stopped. Will still attempt to add job to schedule.");
                (
                    e.get_mut().schedule_with_limit(
                        command,
                        schedule,
                        self.timezone,
                        limit_num_execs,
                    ),
                    true,
                )
            }
        };
        if should_stop_service {
            self.stop()
        };
        if self.active() {
            self.service_running.1.notify_one();
        }
        job_id
    }

    /// Removes a job from the scheduler. Any active executions of this job
    /// will be allowed to complete, but all future jobs will not execute.
    ///
    /// Returns a `DescheduleError` on failure to remove the job.
    pub fn remove_job(&mut self, id: JobId) -> Result<(), job::DescheduleError> {
        let (result, should_stop_service) = match self.job_board.lock() {
            Ok(mut jobs) => (jobs.deschedule(id), false),
            Err(e) => {
                log::error!(target: "scheduler::Scheduler::remove_job", "{e}. Service stopped.");
                (Err(job::DescheduleError::General), true)
            }
        };
        if should_stop_service {
            self.stop()
        };
        result
    }
}

impl<T> Drop for Scheduler<T>
where
    T: TimeZone + Copy + Clone + Send + Sync + 'static,
    T::Offset: Send,
{
    fn drop(&mut self) {
        self.stop();
    }
}
