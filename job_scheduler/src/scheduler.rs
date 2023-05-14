use std::{
    fmt::Debug,
    sync::{mpsc, Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

use chrono::{TimeZone, Utc};
use cron::Schedule;
use futures::future::BoxFuture;

use crate::{job::JobResult, runner, AsyncFn};

mod job_stats {
    use crate::{
        job::{Job, JobResult},
        AsyncFn,
    };
    use chrono::{DateTime, TimeZone, Utc};
    use cron::Schedule;
    use futures::future::BoxFuture;
    use std::{
        cmp::Reverse,
        collections::{BinaryHeap, HashSet},
        fmt::Debug,
    };

    const STARTING_AVAILIABLE_IDS: u32 = 16;

    #[derive(Debug)]
    pub enum DescheduleError {
        AlreadyScheduled,
        JobDoesNotExist,
        General,
    }

    pub enum JobError {
        NoMoreJobs,
        JobFinished,
        ScheduledForDeletion,
    }

    pub struct JobSchedule<T>
    where
        T: TimeZone + Send + Sync + Debug,
    {
        timezone: T,
        now: Option<DateTime<T>>,
        highest_id: Option<u32>,
        available_ids: BinaryHeap<Reverse<u32>>,
        active_jobs: BinaryHeap<Reverse<Job<T>>>,
        scheduled_for_deletion: HashSet<u32>,
    }

    impl<T> JobSchedule<T>
    where
        T: TimeZone + Send + Sync + Debug,
    {
        pub fn new(timezone: T) -> Self {
            Self {
                now: None,
                timezone,
                highest_id: Some(STARTING_AVAILIABLE_IDS - 1),
                available_ids: Self::create_min_heap_with_size(STARTING_AVAILIABLE_IDS),
                active_jobs: BinaryHeap::with_capacity(STARTING_AVAILIABLE_IDS as usize),
                scheduled_for_deletion: HashSet::new(),
            }
        }

        pub fn with_capacity(timezone: T, capacity: u32) -> Self {
            Self {
                now: None,
                timezone,
                highest_id: {
                    if capacity > 0 {
                        Some(capacity)
                    } else {
                        None
                    }
                },
                available_ids: Self::create_min_heap_with_size(capacity),
                active_jobs: BinaryHeap::with_capacity(capacity as usize),
                scheduled_for_deletion: HashSet::new(),
            }
        }

        pub fn schedule<C>(&mut self, command: C, schedule: Schedule, timezone: T) -> u32
        where
            C: AsyncFn + Send + 'static,
        {
            let job = Job::new(
                self.available_ids
                    .pop()
                    .or_else(|| {
                        if let Some(highest_id) = self.highest_id.as_mut() {
                            *highest_id += 1;
                            Some(Reverse(*highest_id))
                        } else {
                            self.highest_id = Some(0);
                            Some(Reverse(0))
                        }
                    })
                    .unwrap()
                    .0,
                command,
                schedule,
                timezone,
            );
            let jid = job.id();
            self.active_jobs.push(Reverse(job));
            jid
        }

        /// Returns the next time that a job should be executed, or
        /// nothing if there are no jobs left.
        /// Will return the current time if the next job is complete.
        pub fn peek_next(&mut self) -> Option<&DateTime<T>> {
            self.active_jobs.peek()?.0.next_exec_time().or_else(|| {
                self.now = Some(Utc::now().with_timezone(&self.timezone));
                self.now.as_ref()
            })
        }

        pub fn try_run_next(&mut self) -> Result<BoxFuture<'static, JobResult>, JobError> {
            let result = match self.active_jobs.pop() {
                Some(mut job) => {
                    if self.scheduled_for_deletion.remove(&job.0.id()) {
                        log::trace!(target: "scheduler::job_stats::JobSchedule::try_run_next", "Job was scheduled for deletion, returning error.");
                        self.available_ids.push(Reverse(job.0.id()));
                        return Err(JobError::ScheduledForDeletion);
                    }
                    if job.0.next_exec_time().is_none() {
                        log::trace!(target: "scheduler::job_stats::JobSchedule::try_run_next", "Job had no more datetimes, is finished, returning error.");
                        self.available_ids.push(Reverse(job.0.id()));
                        return Err(JobError::JobFinished);
                    }
                    let future = job.0.call();
                    log::trace!(target: "scheduler::job_stats::JobSchedule::try_run_next", "Calling job's function, advancing schedule and returning future.");
                    job.0.advance_schedule();
                    self.active_jobs.push(job);
                    return Ok(future);
                }
                None => {
                    log::trace!(target: "scheduler::job_stats::JobSchedule::try_run_next", "No more jobs in the heap, returning error.");
                    Err(JobError::NoMoreJobs)
                }
            };
            result
        }

        pub fn deschedule(&mut self, job_id: u32) -> Result<(), DescheduleError> {
            if self.scheduled_for_deletion.insert(job_id) {
                Ok(())
            } else {
                Err(DescheduleError::AlreadyScheduled)
            }
        }

        fn create_min_heap_with_size(size: u32) -> BinaryHeap<Reverse<u32>> {
            let mut min_heap = BinaryHeap::with_capacity(size as usize);
            (0..size).for_each(|num| min_heap.push(Reverse(num)));
            min_heap
        }
    }
}

enum ProcessManagerState {
    Sleep(Duration),
    Run(BoxFuture<'static, JobResult>),
    Pass,
}

pub struct Scheduler<T>
where
    T: TimeZone + Send + Sync + 'static + Debug,
    T::Offset: Send,
{
    process_manager: Option<JoinHandle<()>>,
    running: Arc<(Mutex<bool>, Condvar)>,
    timezone: T,
    job_stats: Arc<Mutex<job_stats::JobSchedule<T>>>,
}

impl Scheduler<Utc> {
    pub fn new() -> Self {
        Self {
            process_manager: None,
            timezone: Utc,
            running: Arc::new((Mutex::new(false), Condvar::new())),
            job_stats: Arc::new(Mutex::new(job_stats::JobSchedule::new(Utc))),
        }
    }
}

impl<T> Scheduler<T>
where
    T: TimeZone + Send + Sync + 'static + Debug,
    T::Offset: Send,
{
    const MINUTES_IN_AN_HOUR: u64 = 60;
    const SECONDS_IN_A_MINUTE: u64 = 60;

    pub fn with_timezone(timezone: T) -> Self {
        Self {
            process_manager: None,
            timezone: timezone.clone(),
            running: Arc::new((Mutex::new(false), Condvar::new())),
            job_stats: Arc::new(Mutex::new(job_stats::JobSchedule::new(timezone))),
        }
    }

    pub fn start(&mut self) {
        if self.active() {
            return;
        } // DO NOT START NEW THREAD IF ALREADY ACTIVE

        let running = self.running.clone();
        let jobs = self.job_stats.clone();
        let timezone = self.timezone.clone();

        // START
        *self.running.0.lock().unwrap() = true;
        log::info!(target: "scheduler::Scheduler::start", "Starting service.");

        self.process_manager = Some(thread::spawn(move || {
            // Create a new thread and channel.
            // New thread gets receiving channel, curr thread gets sender channel.
            // New thread will hold the tokio async runtime, never -thread- sleep.
            let (sender, reciever) = mpsc::channel::<BoxFuture<'static, JobResult>>();
            let sleep = Arc::new((Mutex::new(()), Condvar::new()));
            let sleep_for_runner = sleep.clone();
            let runner_handle = thread::spawn(move || {
                runner::runner(reciever, sleep_for_runner);
            });

            while *running.0.lock().unwrap() {
                let mut state = ProcessManagerState::Pass;
                let now = Utc::now().with_timezone(&timezone);
                let mut jobs = jobs.lock().unwrap();

                if let Some(exec_time) = jobs.peek_next() {
                    if *exec_time > now {
                        log::debug!(target: "process_manager_thread", "Can't run yet, time is in the future: {:?}.", exec_time);
                        let diff = exec_time.clone() - now.clone();

                        state = ProcessManagerState::Sleep(
                            diff.to_std().unwrap_or(Duration::from_secs(0)),
                        );
                    } else {
                        log::debug!(target: "process_manager_thread", "Attempting to exec job.");
                        match jobs.try_run_next() {
                            Ok(future) => {
                                state = ProcessManagerState::Run(future);
                            }
                            Err(job_stats::JobError::NoMoreJobs) => {}
                            Err(job_stats::JobError::JobFinished) => {}
                            Err(job_stats::JobError::ScheduledForDeletion) => {}
                        }
                    }
                } else {
                    state = ProcessManagerState::Sleep(Duration::from_secs(
                        Self::SECONDS_IN_A_MINUTE * Self::MINUTES_IN_AN_HOUR,
                    ));
                }

                drop(jobs);

                match state {
                    ProcessManagerState::Sleep(duration) => {
                        log::debug!(target: "process_manager_thread", "About to sleep for {:?}.", &duration);
                        drop(
                            running
                                .1
                                .wait_timeout(running.0.lock().unwrap(), duration)
                                .unwrap()
                                .0,
                        );
                        // Wait a little bit after being woken up so main can set `running` if needed.
                        thread::sleep(Duration::from_millis(200));
                    }
                    ProcessManagerState::Run(future) => {
                        log::info!(target: "process_manager_thread", "Running job!");
                        if let Err(e) = sender.send(future) {
                            log::error!(target: "process_manager_thread", "{e}");
                        } else {
                            sleep.1.notify_one();
                        }
                    }
                    ProcessManagerState::Pass => (),
                }
            }
            log::trace!(target: "process_manager_thread", "Ending process manager thread.");

            // Cleanup | TODO: Error handling please
            drop(sender);
            sleep.1.notify_one();
            if let Err(e) = runner_handle.join() {
                log::error!(target: "process_manager_thread", "{:?}", e);
            }
            log::trace!(target: "process_manager_thread", "Leaving closure.");
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

    pub fn add_job<C>(&mut self, command: C, schedule: Schedule) -> u32
    where
        C: AsyncFn + Send + 'static,
    {
        let result = match self.job_stats.lock() {
            Ok(mut jobs) => (
                jobs.schedule(command, schedule, self.timezone.clone()),
                true,
            ),
            Err(mut e) => {
                log::error!(target: "scheduler::Scheduler::add_job", "{}. Service stopped. Will still attempt to add job to schedule.", e.to_string());
                if let Some(handle) = self.process_manager.take() {
                    if let Err(e) = handle.join() {
                        log::error!(target: "scheduler::Scheduler::add_job", "Unable to join process manager thread: {:?}", e);
                    }
                }
                (
                    e.get_mut()
                        .schedule(command, schedule, self.timezone.clone()),
                    false,
                )
            }
        };
        if !result.1 {
            self.stop()
        };
        if self.active() {
            self.running.1.notify_one();
        }
        result.0
    }

    pub fn remove_job(&mut self, id: u32) -> Result<(), job_stats::DescheduleError> {
        let did_an_err_occur = match self.job_stats.lock() {
            Ok(mut jobs) => (jobs.deschedule(id), true),
            Err(e) => {
                log::error!(target: "scheduler::Scheduler::remove_job", "{e}. Service stopped.");
                (Err(job_stats::DescheduleError::General), false)
            }
        };
        if !did_an_err_occur.1 {
            self.stop()
        };
        did_an_err_occur.0
    }
}

impl<T> Drop for Scheduler<T>
where
    T: TimeZone + Send + Sync + 'static + Debug,
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
