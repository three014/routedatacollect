use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashSet},
};

use chrono::{DateTime, TimeZone, Utc};
use cron::Schedule;
use futures::future::BoxFuture;

use crate::{AsyncFn, JobId};

use self::job_internal::Job;
mod job_internal {
    use std::iter::Take;

    use chrono::{DateTime, TimeZone};
    use cron::{OwnedScheduleIterator, Schedule};

    use crate::{AsyncFn, JobId};

    pub struct Job<T>
    where
        T: TimeZone + Send,
    {
        id: JobId,
        next_exec_time: Option<DateTime<T>>,
        schedule: Take<OwnedScheduleIterator<T>>,
        command: Box<dyn AsyncFn + Send + 'static>,
    }

    impl<T> Job<T>
    where
        T: TimeZone + Send,
    {
        pub fn with_limit<C: AsyncFn + Send + 'static>(
            id: JobId,
            command: C,
            schedule: Schedule,
            timezone: T,
            limit: usize,
        ) -> Self {
            let mut schedule = schedule.upcoming_owned(timezone);
            Self {
                id,
                next_exec_time: schedule.next(),
                command: Box::new(command),
                schedule: schedule.take(limit),
            }
        }

        pub fn next_exec_time(&self) -> Option<&DateTime<T>> {
            self.next_exec_time.as_ref()
        }

        pub fn advance_schedule(&mut self) {
            log::trace!(target: "job::Job::advance_schedule", "Last exec time for {}: {:?}", self.id(), self.next_exec_time);
            self.next_exec_time = self.schedule.next();
            log::trace!(target: "job::Job::advance_schedule", "Next exec time for {}: {:?}", self.id(), self.next_exec_time);
        }

        pub fn id(&self) -> u32 {
            self.id
        }
    }

    impl<T> PartialEq for Job<T>
    where
        T: TimeZone + Send,
    {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id && self.next_exec_time == other.next_exec_time
        }
    }

    impl<T> PartialOrd for Job<T>
    where
        T: TimeZone + Send,
    {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            use std::cmp::Ordering::Equal;

            match self.next_exec_time.partial_cmp(&other.next_exec_time) {
                Some(Equal) => return self.id.partial_cmp(&other.id),
                cmp => return cmp,
            }
        }
    }

    impl<T> AsyncFn for Job<T>
    where
        T: TimeZone + Send,
    {
        fn call(&self) -> futures::future::BoxFuture<'static, crate::Result> {
            self.command.call()
        }
    }

    impl<T> Eq for Job<T> where T: TimeZone + Send {}

    impl<T> Ord for Job<T>
    where
        T: TimeZone + Send,
    {
        /// A job only has two comparable features: its id and
        /// its next execution time. We compare the execution times
        /// first, where if both exec times are `None` or `Some` then
        /// we compare the id's and return that result. If `self` has
        /// no more execution times left, then return `Ordering::Less`,
        /// otherwise return `Ordering::Greater`.
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            use std::cmp::Ordering;

            if self.next_exec_time.is_some() && other.next_exec_time.is_some() {
                self.next_exec_time
                    .partial_cmp(&other.next_exec_time)
                    .unwrap()
            } else if self.next_exec_time.is_none() && other.next_exec_time.is_none() {
                self.id.cmp(&other.id)
            } else {
                if self.next_exec_time.is_some() {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
        }
    }
}

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
    T: TimeZone + Send + Sync,
{
    timezone: T,
    now: Option<DateTime<T>>,
    highest_id: Option<JobId>,
    available_ids: BinaryHeap<Reverse<JobId>>,
    active_jobs: BinaryHeap<Reverse<Job<T>>>,
    scheduled_for_deletion: HashSet<JobId>,
}

impl<T> JobSchedule<T>
where
    T: TimeZone + Send + Sync,
{
    pub fn new(timezone: T) -> Self {
        Self::with_capacity(timezone, STARTING_AVAILIABLE_IDS)
    }

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
            scheduled_for_deletion: HashSet::new(),
        }
    }

    pub fn schedule_with_limit<C>(&mut self, command: C, schedule: Schedule, timezone: T, limit: usize) -> JobId
    where
        C: AsyncFn + Send + 'static,
    {
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

    pub fn schedule<C>(&mut self, command: C, schedule: Schedule, timezone: T) -> JobId 
    where
        C: AsyncFn + Send + 'static,
    {
        self.schedule_with_limit(command, schedule, timezone, std::usize::MAX)
    }

    /// Returns the next time that a job should be executed, or
    /// nothing if there are no jobs left.
    /// Will return the current time if the next job is complete.
    pub fn peek_next(&mut self) -> Option<&DateTime<T>> {
        self.active_jobs.peek()?.0.next_exec_time().or_else(|| {
            self.now = Some(Utc::now().with_timezone(&self.timezone) - chrono::Duration::milliseconds(500));
            self.now.as_ref()
        })
    }

    pub fn try_run_next(&mut self) -> Result<(JobId, BoxFuture<'static, crate::Result>), JobError> {
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
                let id = job.0.id();
                self.active_jobs.push(job);
                return Ok((id, future));
            }
            None => {
                log::trace!(target: "scheduler::job_stats::JobSchedule::try_run_next", "No more jobs in the heap, returning error.");
                Err(JobError::NoMoreJobs)
            }
        };
        result
    }

    pub fn deschedule(&mut self, job_id: JobId) -> Result<(), DescheduleError> {
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

    pub fn capacity(&self) -> usize {
        self.available_ids.capacity()
    }
}
