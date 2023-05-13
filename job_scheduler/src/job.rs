use std::{error::Error, fmt::Debug};

use chrono::{DateTime, TimeZone};
use cron::{OwnedScheduleIterator, Schedule};

use crate::AsyncFn;

pub type JobResult = Result<(), Box<dyn Error + Send>>;

pub struct Job<T>
where
    T: TimeZone + Send + Debug,
{
    id: u32,
    next_exec_time: Option<DateTime<T>>,
    schedule: OwnedScheduleIterator<T>,
    command: Box<dyn AsyncFn + Send + 'static>,
}

impl<T> Job<T>
where
    T: TimeZone + Send + Debug,
{
    pub fn new<C: AsyncFn + Send + 'static>(
        id: u32,
        command: C,
        schedule: Schedule,
        timezone: T,
    ) -> Self {
        let mut schedule = schedule.upcoming_owned(timezone);
        Self {
            id,
            next_exec_time: schedule.next(),
            command: Box::new(command),
            schedule,
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
    T: TimeZone + Send + Debug,
{
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.next_exec_time == other.next_exec_time
    }
}

impl<T> PartialOrd for Job<T>
where
    T: TimeZone + Send + Debug,
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
    T: TimeZone + Send + Debug,
{
    fn call(&self) -> futures::future::BoxFuture<'static, JobResult> {
        self.command.call()
    }
}

impl<T> Eq for Job<T> where T: TimeZone + Send + Debug {}

impl<T> Ord for Job<T>
where
    T: TimeZone + Send + Debug,
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
