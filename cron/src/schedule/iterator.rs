use super::Schedule;
use chrono::{DateTime, TimeZone, Utc};

pub struct ScheduleIter<'a, Tz: TimeZone + 'static> {
    schedule: &'a mut Schedule,
    timezone: Tz,
    next: Option<DateTime<Tz>>,
}

pub struct OwnedScheduleIter<Tz: TimeZone + 'static> {
    schedule: Schedule,
    timezone: Tz,
    next: Option<DateTime<Tz>>,
}

impl<'a, Tz: TimeZone + 'static> ScheduleIter<'a, Tz> {
    pub fn new(schedule: &'a mut Schedule, timezone: Tz) -> Self {
        Self {
            schedule,
            timezone,
            next: None,
        }
    }
}

impl<Tz: TimeZone + 'static> OwnedScheduleIter<Tz> {
    pub fn new(schedule: Schedule, timezone: Tz) -> Self {
        Self {
            schedule,
            timezone,
            next: None,
        }
    }
}

impl<Tz: TimeZone + 'static> Iterator for OwnedScheduleIter<Tz> {
    type Item = DateTime<Tz>;

    fn next(&mut self) -> Option<Self::Item> {
        let when = self
            .next
            .get_or_insert_with(|| Utc::now().with_timezone(&self.timezone));
        self.schedule.after(when)
    }
}

impl<'a, Tz: TimeZone + 'static> Iterator for ScheduleIter<'a, Tz> {
    type Item = DateTime<Tz>;

    fn next(&mut self) -> Option<Self::Item> {
        let when = self
            .next
            .get_or_insert_with(|| Utc::now().with_timezone(&self.timezone));
        self.schedule.after(when)
    }
}
