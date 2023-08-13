use crate::{utils::map::SimpleMap, AsyncFn, BoxFuture, Limit, UnsafeAsyncFn};
use chrono::{DateTime, TimeZone, Utc};
use cron::Schedule;
use std::{cmp::Reverse, collections::BinaryHeap};

pub struct JobBoard<T>
where
    T: TimeZone + Send + Sync + 'static,
    T::Offset: Send,
{
    timezone: T,
    now: Option<DateTime<T>>,
    next_id: usize,
    active_jobs: BinaryHeap<Reverse<job::Job<T>>>,
    scheduled_for_deletion: SimpleMap<bool>,
}

impl<T> JobBoard<T>
where
    T: TimeZone + Send + Sync + 'static,
    T::Offset: Send,
{
    const STARTING_AVAILIABLE_IDS: usize = 4;

    pub fn new(timezone: T) -> Self {
        Self::with_capacity(timezone, Self::STARTING_AVAILIABLE_IDS)
    }

    pub fn with_capacity(timezone: T, capacity: usize) -> Self {
        Self {
            now: None,
            timezone,
            next_id: 0,
            active_jobs: BinaryHeap::with_capacity(capacity),
            scheduled_for_deletion: SimpleMap::with_capacity(capacity),
        }
    }

    pub fn schedule<J>(
        &mut self,
        timezone: T,
        schedule: Schedule,
        limit: Option<Limit>,
        job: J,
    ) -> usize
    where
        J: AsyncFn + Clone + Send + 'static,
    {
        let id = self.next_id();
        let job = job::Job::with_limit(id, job, schedule, timezone, limit);
        self.active_jobs.push(Reverse(job));
        id
    }

    pub fn peek_next(&mut self) -> Option<&DateTime<T>> {
        let Reverse(next) = self.active_jobs.peek()?;
        next.next_exec_time().or_else(|| {
            self.now = Some(Utc::now().with_timezone(&self.timezone) - padding());
            self.now.as_ref()
        })
    }

    fn next_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id.checked_add(1).unwrap();
        id
    }

    pub fn try_run_next(&mut self) -> Option<Job> {
        self.active_jobs.pop().and_then(|Reverse(mut job)| {
            let id = job.id();

            if let Some(was_deleted) = self.scheduled_for_deletion.get_mut(id) {
                *was_deleted = true;
                None
            } else if job.next_exec_time().is_none() {
                self.scheduled_for_deletion
                    .entry(id)
                    .and_modify(|was_deleted| {
                        *was_deleted = true;
                    })
                    .or_insert(true);
                None
            } else {
                let command = unsafe { job.call_clone() };
                job.advance_schedule();
                self.active_jobs.push(Reverse(job));
                Some(Job { id, command })
            }
        })
    }

    pub fn deschedule(&mut self, job_id: usize) -> Result<(), error::DescheduleError> {
        if !self.scheduled_for_deletion.contains_key(job_id) {
            self.scheduled_for_deletion.insert(job_id, false);
            Ok(())
        } else {
            Err(error::DescheduleError::AlreadyScheduled(job_id))
        }
    }

    pub fn clear(&mut self) {
        self.active_jobs.clear();
        self.next_id = 0;
        self.now = None;
        self.scheduled_for_deletion.clear();
    }
}

pub mod error {
    use core::fmt;

    #[derive(Debug)]
    pub enum DescheduleError {
        AlreadyScheduled(usize),
        LockPoisioned(String),
    }

    impl fmt::Display for DescheduleError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                DescheduleError::AlreadyScheduled(id) => {
                    write!(f, "Job {id} was already scheduled for deletion.")
                }
                DescheduleError::LockPoisioned(err) => write!(f, "{err}"),
            }
        }
    }

    impl std::error::Error for DescheduleError {}
}

pub struct Job {
    pub id: usize,
    pub command: BoxFuture<'static, crate::Result>,
}

fn padding() -> chrono::Duration {
    const PADDING: i64 = 500;
    chrono::Duration::milliseconds(PADDING)
}

mod job {
    use crate::Limit;
    use chrono::{DateTime, TimeZone, Utc};
    use cron::{OwnedScheduleIterator, Schedule};
    use std::iter::Take;

    use self::async_fn::AsyncFn;

    struct TakeWhile<T>
    where
        T: TimeZone + Send + 'static,
        T::Offset: Send,
    {
        last_date: DateTime<Utc>,
        schedule_iter: OwnedScheduleIterator<T>,
    }

    impl<T> Iterator for TakeWhile<T>
    where
        T: TimeZone + Send + 'static,
        T::Offset: Send,
    {
        type Item = DateTime<T>;

        fn next(&mut self) -> Option<Self::Item> {
            self.schedule_iter
                .next()
                .filter(|date| date.with_timezone(&Utc).timestamp() < self.last_date.timestamp())
        }
    }

    enum ScheduleKind<T>
    where
        T: TimeZone + Send + 'static,
        T::Offset: Send,
    {
        Infinite(OwnedScheduleIterator<T>),
        Finite(Take<OwnedScheduleIterator<T>>),
        Condition(TakeWhile<T>),
    }

    impl<T> Iterator for ScheduleKind<T>
    where
        T: TimeZone + Send + 'static,
        T::Offset: Send,
    {
        type Item = DateTime<T>;

        fn next(&mut self) -> Option<Self::Item> {
            match self {
                Self::Infinite(i) => i.next(),
                Self::Finite(f) => f.next(),
                Self::Condition(c) => c.next(),
            }
        }
    }

    /// The Job item itself. Contains the async function/closure
    /// and the schedule for when this job should be executed. Interprets
    /// the schedule with the supplied timezone, so all future datetimes given
    /// by this job will be of the same timezone.
    pub struct Job<T>
    where
        T: TimeZone + Send + 'static,
        T::Offset: Send,
    {
        id: usize,
        next_exec_time: Option<DateTime<T>>,
        schedule: ScheduleKind<T>,
        command: AsyncFn,
    }

    mod async_fn {
        pub struct AsyncFn {
            function: Box<dyn crate::AsyncFn + Send + 'static>,
        }

        impl AsyncFn {
            pub fn new<F>(f: F) -> Self
            where
                F: crate::AsyncFn + Clone + Send + 'static,
            {
                Self {
                    function: Box::new(f),
                }
            }
        }

        unsafe impl crate::UnsafeAsyncFn for AsyncFn {
            unsafe fn call_clone(&self) -> crate::BoxFuture<'static, crate::Result> {
                // SAFETY: self.function is non-null since it is wrapped in
                // a Box, and that function is never accessed or written to itself,
                // since AsyncFn clones the value before calling the function.
                let copied_ptr: *const _ = &*self.function;
                (*copied_ptr).call()
            }
        }
    }

    impl<T> Job<T>
    where
        T: TimeZone + Send + 'static,
        T::Offset: Send,
    {
        pub fn with_limit<J>(
            id: usize,
            job_fn: J,
            schedule: Schedule,
            timezone: T,
            limit: Option<Limit>,
        ) -> Self
        where
            J: crate::AsyncFn + Clone + Send + 'static,
        {
            let mut schedule = schedule.upcoming_owned(timezone);
            Self {
                id,
                next_exec_time: schedule.next(),
                command: async_fn::AsyncFn::new(job_fn),
                schedule: match limit {
                    Some(Limit::NumTimes(n)) => {
                        ScheduleKind::Finite(schedule.take(n.saturating_sub(1) as usize))
                    }
                    Some(Limit::EndDate(last_date)) => ScheduleKind::Condition(TakeWhile {
                        last_date: Utc.from_local_datetime(&last_date).unwrap(),
                        schedule_iter: schedule,
                    }),
                    None => ScheduleKind::Infinite(schedule),
                },
            }
        }

        /// Returns the next execution time of this job. This will always
        /// occur if the job was created with a `Limit::None`. 
        /// Returns `None` if the job reaches the limit that was specified with either
        /// `Limit::EndDate` or `Limit::NumTimes`.
        #[inline]
        pub fn next_exec_time(&self) -> Option<&DateTime<T>> {
            self.next_exec_time.as_ref()
        }

        /// Advances the schedule of this job to the next possible `chrono::DateTime`, if it exists.
        /// Use `next_exec_time` to see check the actual datetime.
        #[inline]
        pub fn advance_schedule(&mut self) {
            self.next_exec_time = self.schedule.next();
        }

        /// Returns the id given to this job.
        #[inline]
        pub fn id(&self) -> usize {
            self.id
        }
    }

    impl<T> PartialEq for Job<T>
    where
        T: TimeZone + Send + 'static,
        T::Offset: Send,
    {
        fn eq(&self, other: &Self) -> bool {
            self.id() == other.id() && self.next_exec_time == other.next_exec_time
        }
    }

    impl<T> PartialOrd for Job<T>
    where
        T: TimeZone + Send + 'static,
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

    unsafe impl<T> crate::UnsafeAsyncFn for Job<T>
    where
        T: TimeZone + Send + 'static,
        T::Offset: Send,
    {
        unsafe fn call_clone(&self) -> crate::BoxFuture<'static, crate::Result> {
            self.command.call_clone()
        }
    }

    impl<T> Eq for Job<T>
    where
        T: TimeZone + Send + 'static,
        T::Offset: Send,
    {
    }

    impl<T> Ord for Job<T>
    where
        T: TimeZone + Send + 'static,
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
}
