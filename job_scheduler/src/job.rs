use self::job_internal::Job;
use crate::{runner::RunningJobs, AsyncFn, JobId, Limit};
use chrono::{DateTime, TimeZone, Utc};
use cron::Schedule;
use futures::future::BoxFuture;
use fxhash::FxHasher32;
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    hash::BuildHasherDefault,
    sync::{Arc, Mutex},
};
mod job_internal {
    use crate::{AsyncFn, JobId, Limit};
    use chrono::{DateTime, TimeZone, Utc};
    use cron::Schedule;

    /// The Job item itself. Contains the async function/closure
    /// and the schedule for when this job should be executed. Interprets
    /// the schedule with the supplied timezone, so all future datetimes given
    /// by this job will be of the same timezone.
    pub struct Job<T>
    where
        T: TimeZone + Send,
        T::Offset: Send,
    {
        id: JobId,
        next_exec_time: Option<DateTime<T>>,
        schedule: Box<dyn Iterator<Item = DateTime<T>> + Send>,
        command: Box<dyn AsyncFn + Send + 'static>,
    }

    impl<T> Job<T>
    where
        T: TimeZone + Clone + Copy + Send + 'static,
        T::Offset: Send,
    {
        /// Creates a new job struct with the supplied
        /// id, ['job_scheduler::AsyncFn'], schedule, timezone,
        /// and limit
        pub fn with_limit<C: AsyncFn + Send + 'static>(
            id: JobId,
            command: C,
            schedule: Schedule,
            timezone: T,
            limit: Limit,
        ) -> Self {
            let mut schedule = schedule.upcoming_owned(timezone);
            Self {
                id,
                next_exec_time: schedule.next(),
                command: Box::new(command),
                schedule: match limit {
                    Limit::None => Box::new(schedule),
                    Limit::NumTimes(num_times) => {
                        Box::new(schedule.take(num_times.checked_sub(1).unwrap_or_default()))
                    }
                    Limit::EndDate(end_date) => Box::new(schedule.take_while(move |date_time| {
                        date_time.with_timezone(&Utc).timestamp()
                            < Utc.from_local_datetime(&end_date).unwrap().timestamp()
                    })),
                },
            }
        }

        /// Returns the next execution time of this job. This will always
        /// occur if the job was created with a `Limit::None`.
        ///
        /// Returns `None` if the job reaches the limit that was specified with either
        /// `Limit::EndDate` or `Limit::NumTimes`.
        pub fn next_exec_time(&self) -> Option<&DateTime<T>> {
            self.next_exec_time.as_ref()
        }

        /// Advances the schedule of this job to the next possible `chrono::DateTime`, if it exists.
        /// Use `next_exec_time` to see check the actual datetime.
        pub fn advance_schedule(&mut self) {
            log::trace!(target: "job::Job::advance_schedule", "Last exec time for {}: {:?}", self.id(), self.next_exec_time);
            self.next_exec_time = self.schedule.next();
            log::trace!(target: "job::Job::advance_schedule", "Next exec time for {}: {:?}", self.id(), self.next_exec_time);
        }

        /// Returns the id given to this job.
        pub fn id(&self) -> JobId {
            self.id
        }
    }

    impl<T> PartialEq for Job<T>
    where
        T: TimeZone + Send,
        T::Offset: Send,
    {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id && self.next_exec_time == other.next_exec_time
        }
    }

    impl<T> PartialOrd for Job<T>
    where
        T: TimeZone + Send,
        T::Offset: Send,
    {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            use std::cmp::Ordering::Equal;

            match self.next_exec_time.partial_cmp(&other.next_exec_time) {
                Some(Equal) => self.id.partial_cmp(&other.id),
                cmp => cmp,
            }
        }
    }

    impl<T> AsyncFn for Job<T>
    where
        T: TimeZone + Send,
        T::Offset: Send,
    {
        fn call(&self) -> futures::future::BoxFuture<'static, crate::Result> {
            self.command.call()
        }
    }

    impl<T> Eq for Job<T>
    where
        T: TimeZone + Send,
        T::Offset: Send,
    {
    }

    impl<T> Ord for Job<T>
    where
        T: TimeZone + Send,
        T::Offset: Send,
    {
        /// A job only has two comparable features: its id and
        /// its next execution time. We compare the execution times
        /// first, but if both exec times are `None` or `Some` then
        /// we compare the id's and return that result. If `self` has
        /// no more execution times left, then return `Ordering::Less`,
        /// otherwise return `Ordering::Greater`.
        ///
        /// This is so the scheduler can sooner sift out the jobs that
        /// have already completed, leaving the queue filled with only
        /// available jobs.
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            use std::cmp::Ordering;

            if self.next_exec_time.is_some() && other.next_exec_time.is_some() {
                self.next_exec_time
                    .partial_cmp(&other.next_exec_time)
                    .unwrap()
            } else if self.next_exec_time.is_none() && other.next_exec_time.is_none() {
                self.id.cmp(&other.id)
            } else if self.next_exec_time.is_some() {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::Job;
        use crate::Limit;
        use chrono::Utc;

        #[test]
        fn id_matches_what_was_given() {
            let id = rand::random();

            let job = Job::with_limit(
                id,
                || async { Ok(()) },
                "00 * * * * *".parse().unwrap(),
                Utc,
                Limit::NumTimes(0),
            );
            assert_eq!(id, job.id());
        }

        #[test]
        fn job_without_command_should_be_less_than_job_with_command() {
            let mut job1 = Job::with_limit(
                1,
                || async { Ok(()) },
                "00 * * * * *".parse().unwrap(),
                Utc,
                Limit::NumTimes(0),
            );

            let job2 = Job::with_limit(
                2,
                || async { Ok(()) },
                "00 * * * * *".parse().unwrap(),
                Utc,
                Limit::NumTimes(0),
            );

            job1.advance_schedule();

            assert!(job1 < job2);
        }

        #[test]
        fn job_with_sooner_exec_time_should_be_less_than_other_job() {
            let mut sooner_job = Job::with_limit(
                1,
                || async { Ok(()) },
                "00 * * * * *".parse().unwrap(),
                Utc,
                Limit::NumTimes(3),
            );

            let mut later_job = Job::with_limit(
                2,
                || async { Ok(()) },
                "00 * * * * *".parse().unwrap(),
                Utc,
                Limit::NumTimes(3),
            );

            sooner_job.advance_schedule();
            later_job.advance_schedule();
            later_job.advance_schedule();

            assert!(sooner_job < later_job);
        }
    }
}

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

/// Stores all the jobs and contains the logic
/// for scheduling, descheduling, and selecting
/// the next job to execute.
///
/// Currently uses a `std::collections::BinaryHeap`
/// to store available jobs, but this is subject
/// to change if I can learn how to implement a
/// `CalendarQueue`.
pub struct JobBoard<T>
where
    T: TimeZone + Send + Sync,
    T::Offset: Send,
{
    timezone: T,
    now: Option<DateTime<T>>,
    highest_id: Option<JobId>,
    available_ids: BinaryHeap<Reverse<JobId>>,
    active_jobs: BinaryHeap<Reverse<Job<T>>>,
    scheduled_for_deletion: HashMap<JobId, bool, BuildHasherDefault<FxHasher32>>,
    running_jobs: Arc<Mutex<RunningJobs>>,
}

impl<T> JobBoard<T>
where
    T: TimeZone + Copy + Clone + Send + Sync + 'static,
    T::Offset: Send,
{
    const STARTING_AVAILIABLE_IDS: u32 = 16;

    /// Creates a new `JobBoard<T>` with a capacity of 16.
    pub fn new(timezone: T) -> Self {
        Self::with_capacity(timezone, Self::STARTING_AVAILIABLE_IDS)
    }

    /// Creates a new `JobBoard<T>` with the specified capacity.
    ///
    /// The capacity is a `u32` because the job ids are also `u32` values,
    /// therefore limiting the maximum number of unique jobs in the
    /// job board to `u32::MAX`.
    pub fn with_capacity(timezone: T, capacity: u32) -> Self {
        Self {
            now: None,
            timezone,
            highest_id: {
                if capacity > 0 {
                    Some(capacity - 1)
                } else {
                    None
                }
            },
            available_ids: Self::create_min_heap_with_size(capacity),
            active_jobs: BinaryHeap::with_capacity(capacity as usize),
            scheduled_for_deletion: HashMap::with_hasher(BuildHasherDefault::default()),
            running_jobs: Arc::new(Mutex::new(RunningJobs::with_capacity(
                (capacity / 2) as usize,
            ))),
        }
    }

    pub fn schedule_with_limit<C>(
        &mut self,
        command: C,
        schedule: Schedule,
        timezone: T,
        limit: Limit,
    ) -> JobId
    where
        C: AsyncFn + Send + 'static,
    {
        if let Ok(running_jobs) = self.running_jobs.lock() {
            self.scheduled_for_deletion.retain(|id, was_removed| {
                if *was_removed && !running_jobs.contains(id) {
                    self.available_ids.push(Reverse(*id));
                    false
                } else {
                    true
                }
            });
        }

        let job = Job::with_limit(
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
            limit,
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
            self.now = Some(
                Utc::now().with_timezone(&self.timezone) - chrono::Duration::milliseconds(500),
            );
            self.now.as_ref()
        })
    }

    /// Attempts to run the next available `Job`. On success,
    /// returns a tuple containing: 1. The `JobId` of the job that was
    /// just run, and 2. The `BoxFuture` of the job itself, which should
    /// be passed to an async runtime to be polled.
    ///
    /// If the next job can't be run for some reason, the function
    /// returns a `JobError` containing the reason for failure,
    /// but also mutates the internal data structure so that future calls
    /// to this function can yield the next available job.
    pub fn try_run_next(&mut self) -> Result<(JobId, BoxFuture<'static, crate::Result>), JobError> {
        let result = match self.active_jobs.pop() {
            Some(mut job) => {
                let id = job.0.id();

                // Jobs that are scheduled for deletion won't make it to the runner
                if let Some(was_deleted) = self.scheduled_for_deletion.get_mut(&id) {
                    *was_deleted = true;
                    log::trace!(target: "scheduler::job_stats::JobSchedule::try_run_next", "Job was scheduled for deletion, returning error.");
                    Err(JobError::ScheduledForDeletion)
                } else if job.0.next_exec_time().is_none() {
                    self.scheduled_for_deletion.insert(id, true);
                    log::trace!(target: "scheduler::job_stats::JobSchedule::try_run_next", "Job had no more datetimes, is finished, returning error.");
                    Err(JobError::JobFinished)
                } else {
                    // Create the future
                    let future = job.0.call();
                    log::trace!(target: "scheduler::job_stats::JobSchedule::try_run_next", "Calling job's function, advancing schedule and returning future.");
                    job.0.advance_schedule();
                    self.active_jobs.push(job);
                    Ok((id, future))
                }
            }
            None => {
                log::trace!(target: "scheduler::job_stats::JobSchedule::try_run_next", "No more jobs in the min heap, returning error.");
                Err(JobError::NoMoreJobs)
            }
        };
        result
    }

    /// Marks a job for removal, returning a `DescheduleError` if this fails
    /// for any reason. On success, the next time the internal clock chooses
    /// this job to run, it will instead delete the job.
    pub fn deschedule(&mut self, job_id: JobId) -> Result<(), DescheduleError> {
        if let std::collections::hash_map::Entry::Vacant(e) =
            self.scheduled_for_deletion.entry(job_id)
        {
            e.insert(false);
            Ok(())
        } else {
            Err(DescheduleError::AlreadyScheduled)
        }
    }

    pub fn currently_running(&self) -> Arc<Mutex<RunningJobs>> {
        self.running_jobs.clone()
    }

    fn create_min_heap_with_size(size: u32) -> BinaryHeap<Reverse<u32>> {
        let mut min_heap = BinaryHeap::with_capacity(size as usize);
        (0..size).for_each(|num| min_heap.push(Reverse(num)));
        min_heap
    }
}
