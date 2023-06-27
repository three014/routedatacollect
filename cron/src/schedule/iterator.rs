use super::Schedule;
use chrono::{DateTime, TimeZone};

pub struct ScheduleIter<'a, Tz: TimeZone> {
    schedule: &'a mut Schedule,
    next: Option<DateTime<Tz>>,
}

pub struct OwnedScheduleIter<Tz: TimeZone> {
    schedule: Schedule,
    next: Option<DateTime<Tz>>,
}

impl<'a, Tz: TimeZone> ScheduleIter<'a, Tz> {
    pub fn new(schedule: &'a mut Schedule, next: Option<DateTime<Tz>>) -> Self {
        Self { schedule, next }
    }
}

impl<Tz: TimeZone> OwnedScheduleIter<Tz> {
    pub fn new(schedule: Schedule, next: Option<DateTime<Tz>>) -> Self {
        Self { schedule, next }
    }
}

impl<'a, Tz: TimeZone + 'static> Iterator for ScheduleIter<'a, Tz> {
    type Item = DateTime<Tz>;

    fn next(&mut self) -> Option<Self::Item> {
        let now = self.next.take()?;
        self.next = self.schedule.next(&now.timezone());
        Some(now)
    }
}

impl<Tz: TimeZone + 'static> Iterator for OwnedScheduleIter<Tz> {
    type Item = DateTime<Tz>;

    fn next(&mut self) -> Option<Self::Item> {
        let now = self.next.take()?;
        self.next = self.schedule.next(&now.timezone());
        Some(now)
    }
}
