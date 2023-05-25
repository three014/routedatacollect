use std::{
    sync::{mpsc, Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

use chrono::{TimeZone, Utc};
use cron::Schedule;
use futures::future::BoxFuture;

use crate::{job, runner, AsyncFn, JobId};

enum ProcessManagerState {
    Sleep(Duration),
    Run((JobId, BoxFuture<'static, crate::Result>)),
    Pass,
}

pub struct Scheduler<T>
where
    T: TimeZone + Copy + Clone + Send + Sync + 'static,
    T::Offset: Send,
{
    process_manager: Option<JoinHandle<()>>,
    running: Arc<(Mutex<bool>, Condvar)>,
    timezone: T,
    job_stats: Arc<Mutex<job::JobBoard<T>>>,
}

impl Scheduler<Utc> {
    pub fn new() -> Self {
        Self {
            process_manager: None,
            timezone: Utc,
            running: Arc::new((Mutex::new(false), Condvar::new())),
            job_stats: Arc::new(Mutex::new(job::JobBoard::new(Utc))),
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

    pub fn with_timezone(timezone: T) -> Self {
        Self {
            process_manager: None,
            timezone,
            running: Arc::new((Mutex::new(false), Condvar::new())),
            job_stats: Arc::new(Mutex::new(job::JobBoard::new(timezone))),
        }
    }

    pub fn start(&mut self) {
        if self.active() {
            return;
        } // DO NOT START NEW THREAD IF ALREADY ACTIVE

        let running = self.running.clone();
        let jobs = self.job_stats.clone();
        let lock = self.job_stats.lock().unwrap();
        let running_jobs_report = lock.currently_running();
        drop(lock);

        // START
        *self.running.0.lock().unwrap() = true;
        log::info!(target: "scheduler::Scheduler::start", "Starting service.");

        self.process_manager = Some(thread::spawn(move || {
            // Create a new thread and channel.
            // New thread gets receiving channel, curr thread gets sender channel.
            let (sender, reciever) = mpsc::channel::<(JobId, BoxFuture<'static, crate::Result>)>();
            let sleep = Arc::new((Mutex::new(()), Condvar::new()));
            let sleep_for_runner = sleep.clone();
            let runner_handle = thread::spawn(move || {
                runner::runner(reciever, sleep_for_runner, running_jobs_report);
            });

            while *running.0.lock().unwrap() {
                let mut state = ProcessManagerState::Pass;
                let mut jobs = jobs.lock().unwrap();

                if let Some(exec_time) = jobs.peek_next() {
                    let now = Utc::now();
                    let then = exec_time.with_timezone(&Utc);
                    log::debug!("now: {:?}, then: {:?}", &now, &then);
                    if then > now {
                        log::debug!(target: "scheduler::process_manager_thread", "Can't run yet, time is in the future: {:?}.", then);
                        let diff = then - now;

                        state = ProcessManagerState::Sleep(
                            diff.to_std().unwrap_or(Duration::from_secs(0)),
                        );
                    } else {
                        log::debug!(target: "scheduler::process_manager_thread", "Attempting to exec job.");
                        if let Ok(job) = jobs.try_run_next() {
                            state = ProcessManagerState::Run(job);
                        } else {
                            log::debug!(target: "scheduler::process_manager_thread", "Couldn't run job.");
                        }
                    }
                } else {
                    state =
                        ProcessManagerState::Sleep(Duration::from_secs(Self::SECONDS_IN_AN_HOUR));
                }

                drop(jobs);

                match state {
                    ProcessManagerState::Sleep(duration) => {
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
                        thread::sleep(Duration::from_millis(Self::PADDING));
                    }
                    ProcessManagerState::Run(job) => {
                        log::info!(target: "scheduler::process_manager_thread", "Running job (id={})!", job.0);
                        if let Err(e) = sender.send(job) {
                            log::error!(target: "scheduler::process_manager_thread", "{e}. Attempting to stop process manager.");
                            *running.0.lock().unwrap() = false; // Stop loop
                        } else {
                            sleep.1.notify_one();
                        }
                    }
                    ProcessManagerState::Pass => (),
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

    pub fn stop(&mut self) {
        if !self.active() {
            return;
        }
        log::info!(target: "scheduler::Scheduler::stop", "Stopping service, waiting for all processes to finish.");
        self.running.1.notify_one();
        *self.running.0.lock().unwrap() = false;
        if let Some(handle) = self.process_manager.take() {
            if let Err(e) = handle.join() {
                log::error!(target: "scheduler::Scheduler::stop", "Unable to join process manager thread during shutdown: {:?}", e);
            }
        }
        log::info!(target: "scheduler::Scheduler::stop", "Stopped.");
    }

    pub fn restart(&mut self) {
        log::info!(target: "schedule::Scheduler::restart", "Restarting service.");
        self.stop();
        self.start();
    }

    pub fn active(&self) -> bool {
        self.process_manager.is_some() && *self.running.0.lock().unwrap()
    }

    pub fn add_job<C>(
        &mut self,
        command: C,
        schedule: Schedule,
        limit_num_execs: crate::Limit,
    ) -> JobId
    where
        C: AsyncFn + Send + 'static,
    {
        let (job_id, should_stop_service) = match self.job_stats.lock() {
            Ok(mut jobs) => (
                jobs.schedule_with_limit(command, schedule, self.timezone, limit_num_execs),
                false,
            ),
            Err(mut e) => {
                log::error!(target: "scheduler::Scheduler::add_job", "{e}. Service stopped. Will still attempt to add job to schedule.");
                (
                    e.get_mut()
                        .schedule(command, schedule, self.timezone),
                    true,
                )
            }
        };
        if should_stop_service {
            self.stop()
        };
        if self.active() {
            self.running.1.notify_one();
        }
        job_id
    }

    /// Removes a job from the scheduler. Any active executions of this job
    /// will be allowed to complete, but all future jobs will not execute
    pub fn remove_job(&mut self, id: JobId) -> Result<(), job::DescheduleError> {
        let (result, should_stop_service) = match self.job_stats.lock() {
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

/* Problem: I want to be able to add jobs to a collection of some kind,
 * so that the scheduler can determine which jobs to add to the queue
 * for the clock to run.
 *
 * What is a job? A job is some sort of data block that contains
 * the schedule that it should run, along with the function/task
 * to run at every point.
 *
 * How should that schedule be stored? I might have to create a new struct
 * to store a schedule.
 * - It could hold optional values to repeat every
 *   - minute
 *   - hour
 *   - day or range of days of the month
 *   - day or range of days of the week
 *   - month
 * - Wait. Maybe I can just use the cron parser?
 * - I CAN
 *
 * I may have the schedule part, but I still need a way to order the schedules so that
 * I always have the next soonest job to be executed
 *
 * I have learned of the Franta-Maly event list. I am going to learn about it and see if
 * I can use it for my scheduler.
 * - It's very confusing, but apparently runs in O(sqrt(n)-ish) time? But also, I learned
 *   of another queue called the Calendar Queue.
 * - I like the calendar queue better, although I still don't get it.
 *
 * I went with a standard binary heap. I felt like if I spent too much time on the other
 * implementations then I'd never get anywhere, and the heap should be okay for the
 * small amount of jobs I'd be giving it.
 */
