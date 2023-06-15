use self::{
    date::{DayCache, Days, Months},
    time::{Hours, Minutes, Seconds},
};
use crate::schedule::iterator::CopyRing;

use super::Error;

mod date {
    use crate::schedule::iterator::CopyRing;

    #[derive(Clone, Debug)]
    pub enum Days {
        Both {
            month: CopyRing<u8>,
            week: (CopyRing<u8>, DayCache),
        },
        Month(CopyRing<u8>),
        Week((CopyRing<u8>, DayCache)),
    }

    #[derive(Clone, Debug)]
    pub struct Months(CopyRing<u8>);

    #[derive(Clone, Debug, Default)]
    pub struct DayCache {
        last_month_day: u8,
        last_weekday: u8,
        last_used: LastUsed,
    }

    #[derive(Clone, Copy, Debug, Default)]
    pub enum LastUsed {
        #[default]
        Week,
        Month,
        Both,
    }

    impl DayCache {
        pub fn set(&mut self, new_summary: DayCache) {
            self.last_month_day = new_summary.last_month_day;
            self.last_used = new_summary.last_used;
            self.last_weekday = new_summary.last_weekday;
        }

        pub(super) fn last_month_day(&self) -> u8 {
            self.last_month_day
        }

        pub(super) fn last_weekday(&self) -> u8 {
            self.last_weekday
        }

        pub(super) fn last_used(&self) -> LastUsed {
            self.last_used
        }

        pub(super) fn set_last_used(&mut self, last_used: LastUsed) {
            self.last_used = last_used;
        }
    }

    impl Days {
        pub fn first_after(
            &mut self,
            day_of_month: u8,
            day_of_week: u8, /* Sunday = 0 -----> Saturday = 6 */
            hours_overflow: bool,
            month: u8,
            year: u32,
        ) -> (u8, bool) {
            use std::cmp::Ordering;
            let days_in_curr_month = crate::days_in_a_month(month, year);
            match self {
                Days::Both { month, week } => {
                    let (week, summary) = week;
                    let next_day_week = Self::next_day_of_the_week(
                        week,
                        hours_overflow,
                        day_of_week,
                        day_of_month,
                        days_in_curr_month,
                    );
                    let next_day_month = Self::next_day_of_the_month(
                        month,
                        hours_overflow,
                        day_of_month,
                        days_in_curr_month,
                    );

                    match (next_day_week.1, next_day_month.1) {
                        (true, true) | (false, false) => {
                            match next_day_month.0.cmp(&next_day_week.0) {
                                Ordering::Less => {
                                    summary.last_used = LastUsed::Month;
                                    summary.last_month_day = next_day_month.0;
                                    summary.last_weekday = next_weekday_from_last(
                                        day_of_week,
                                        next_day_month.0,
                                        days_in_curr_month,
                                        day_of_month,
                                    );
                                    next_day_month
                                }
                                Ordering::Equal => {
                                    summary.last_used = LastUsed::Both;
                                    summary.last_month_day = next_day_month.0;
                                    summary.last_weekday = week.peek().unwrap();
                                    next_day_month
                                } // Doesn't matter which one to return
                                Ordering::Greater => {
                                    summary.last_used = LastUsed::Week;
                                    summary.last_month_day = next_day_week.0;
                                    summary.last_weekday = week.peek().unwrap();
                                    next_day_week
                                }
                            }
                        }
                        (true, false) => {
                            // The month day was sooner, commit to the month day
                            summary.last_used = LastUsed::Month;
                            summary.last_month_day = next_day_month.0;
                            summary.last_weekday = next_weekday_from_last(
                                day_of_week,
                                next_day_month.0,
                                days_in_curr_month,
                                day_of_month,
                            );
                            next_day_month
                        }
                        (false, true) => {
                            // The weekday was sooner, commit to the weekday
                            summary.last_used = LastUsed::Week;
                            summary.last_month_day = next_day_week.0;
                            summary.last_weekday = week.peek().unwrap();
                            next_day_week
                        }
                    }
                }
                Days::Month(month) => Self::next_day_of_the_month(
                    month,
                    hours_overflow,
                    day_of_month,
                    days_in_curr_month,
                ),
                Days::Week((week, summary)) => {
                    let next = Self::next_day_of_the_week(
                        week,
                        hours_overflow,
                        day_of_week,
                        day_of_month,
                        days_in_curr_month,
                    );
                    summary.last_used = LastUsed::Week;
                    summary.last_month_day = next.0;
                    summary.last_weekday = week.peek().unwrap();
                    next
                }
            }
        }

        pub fn next(&mut self, hours_overflow: bool, curr_month: u8, curr_year: u32) -> (u8, bool) {
            use std::cmp::Ordering;
            let weekday_to_month_day =
                |weekday: u8, last_weekday: u8, days_in_curr_month: u8| -> (u8, bool) {
                    let days_since_last =
                        Self::num_weekdays_since(last_weekday as i8, weekday as i8);
                    let next_day_of_the_month = last_weekday + days_since_last;
                    let overflow = next_day_of_the_month > days_in_curr_month;
                    let month_day = if overflow {
                        next_day_of_the_month - days_in_curr_month
                    } else {
                        next_day_of_the_month
                    };
                    (month_day, overflow)
                };
            let days_in_curr_month = crate::days_in_a_month(curr_month, curr_year);
            match self {
                Days::Both { month, week } => {
                    let (week, summary) = week;

                    // We're checking a couple different things:
                    // - Was there overflow from the hours
                    // - Which field has the soonest day
                    //
                    // Now, if there was no overflow from the hours,
                    // then we're not advancing any field, so there'd
                    // be no change to which field we used last.
                    // I don't even think we'd have to update the
                    // summary, since the days wouldn't change
                    //
                    // If there was overflow from the hours, then we really
                    // gotta do some calculations. We look at which field
                    // we used last and advance that field by one, checking
                    // for overflow there. For the other field, we just peek
                    // the next value, and assume no overflow.
                    //
                    // From there, we compare the next days like we
                    // did for the `first_after` method, and the winner
                    // goes into the summary. Then we return the value too.

                    hours_overflow
                        .then(|| {
                            let state = match summary.last_used() {
                                LastUsed::Week => {
                                    let week = week.next().unwrap();
                                    let month = (month.peek().unwrap(), false);
                                    (month, week)
                                }
                                LastUsed::Month => {
                                    let month = month.checked_next().unwrap();
                                    let week = week.peek().unwrap();
                                    (month, week)
                                }
                                LastUsed::Both => {
                                    let month = month.checked_next().unwrap();
                                    let week = week.next().unwrap();
                                    (month, week)
                                }
                            };
                            let ((monthday, monthday_overflow), weekday) = state;

                            // Week
                            let (next_day_of_the_month, weekday_overflow) = weekday_to_month_day(
                                weekday,
                                summary.last_weekday(),
                                days_in_curr_month,
                            );
                            let week_summary = DayCache {
                                last_month_day: next_day_of_the_month,
                                last_weekday: weekday,
                                last_used: LastUsed::Week,
                            };

                            // Month
                            let next_day_of_the_week = next_weekday_from_last(
                                summary.last_weekday(),
                                monthday,
                                days_in_curr_month,
                                summary.last_month_day(),
                            );
                            let mut month_summary = DayCache {
                                last_month_day: monthday,
                                last_weekday: next_day_of_the_week,
                                last_used: LastUsed::Month,
                            };

                            // Compare
                            match (monthday_overflow, weekday_overflow) {
                                (true, true) | (false, false) => {
                                    match monthday.cmp(&next_day_of_the_month) {
                                        Ordering::Less => {
                                            summary.set(month_summary);
                                            (summary.last_month_day(), monthday_overflow)
                                        }
                                        Ordering::Equal => {
                                            month_summary.set_last_used(LastUsed::Both);
                                            summary.set(month_summary);
                                            (summary.last_month_day(), monthday_overflow)
                                        }
                                        Ordering::Greater => {
                                            summary.set(week_summary);
                                            (summary.last_month_day(), weekday_overflow)
                                        }
                                    }
                                }
                                (true, false) => {
                                    // The weekday was sooner
                                    summary.set(week_summary);
                                    (summary.last_month_day(), weekday_overflow)
                                }
                                (false, true) => {
                                    // The monthday was sooner
                                    summary.set(month_summary);
                                    (summary.last_month_day(), monthday_overflow)
                                }
                            }
                        })
                        .unwrap_or((summary.last_month_day(), false))
                }
                Days::Month(month) => hours_overflow
                    .then(|| month.checked_next().unwrap())
                    .unwrap_or((month.peek().unwrap(), false)),
                Days::Week((week, summary)) => hours_overflow
                    .then(|| {
                        let next_day_of_the_week = week.next().unwrap();
                        let (next, overflow) = weekday_to_month_day(
                            next_day_of_the_week,
                            summary.last_weekday(),
                            days_in_curr_month,
                        );
                        let week_summary = DayCache {
                            last_month_day: next,
                            last_weekday: next_day_of_the_week,
                            last_used: LastUsed::Week,
                        };
                        summary.set(week_summary);
                        (summary.last_month_day(), overflow)
                    })
                    .unwrap_or((summary.last_month_day(), false)),
            }
        }

        fn next_day_of_the_month(
            days: &mut CopyRing<u8>,
            hours_overflow: bool,
            day_of_month: u8,
            days_in_curr_month: u8,
        ) -> (u8, bool) {
            let mut found = false;
            days.reset();
            for day in days.one_cycle() {
                if day >= day_of_month {
                    found = true;
                    break;
                }
            }
            if hours_overflow {
                days.rotate_left(1)
            }
            if found {
                days.rotate_right(1);
                let next = days.peek().unwrap();
                if next > days_in_curr_month {
                    days.reset();
                    (days.peek().unwrap(), true)
                } else {
                    (next, false)
                }
            } else {
                (days.peek().unwrap(), true)
            }
        }

        fn next_day_of_the_week(
            days: &mut CopyRing<u8>,
            hours_overflow: bool,
            day_of_week: u8,
            day_of_month: u8,
            days_in_curr_month: u8,
        ) -> (u8, bool) {
            let mut found = false;
            days.reset();
            for weekday in days.one_cycle() {
                if weekday >= day_of_week {
                    found = true;
                    break;
                }
            }
            if found {
                days.rotate_right(1)
            }
            if hours_overflow {
                days.rotate_left(1)
            }
            let weekday = days.peek().unwrap();
            let days_since_now = Self::num_weekdays_since(day_of_week as i8, weekday as i8);
            let next_day_unmodded = day_of_month + days_since_now;
            if next_day_unmodded > days_in_curr_month {
                (next_day_unmodded % days_in_curr_month, true)
            } else {
                (next_day_unmodded, false)
            }
        }

        pub fn num_weekdays_since(first_weekday: i8, second_weekday: i8) -> u8 {
            let days_in_a_week = 7i8;
            let diff = days_in_a_week + second_weekday - first_weekday;
            (diff % days_in_a_week) as u8
        }
    }

    /// # Currently doesn't work
    ///
    /// Works if the next day is from the next month,
    /// but fails if the next day is in the previous month.
    ///
    /// Even worse, this function doesn't factor in
    /// what happens if the next day is more than one month
    /// away.
    pub(crate) fn next_weekday_from_last(
        last_weekday: u8,
        next_day_month: u8,
        days_in_curr_month: u8,
        last_day_of_month: u8,
    ) -> u8 {
        let days_in_a_week = 7;
        (last_weekday + (next_day_month + days_in_curr_month - last_day_of_month)) % days_in_a_week
    }

    impl Months {
        pub fn new(copy_ring: CopyRing<u8>) -> Self {
            Self(copy_ring)
        }

        pub fn first_after(&mut self, day_overflow: bool, month: u8) -> (u8, bool) {
            let mut found = false;
            let mut overflow = false;
            self.0.reset();
            for other_month in self.0.one_cycle().checked() {
                if other_month.0 >= month {
                    found = true;
                    overflow = other_month.1;
                    break;
                }
            }
            if day_overflow {
                overflow = self.0.checked_next().unwrap().1 || overflow
            }
            if found {
                self.0.rotate_right(1);
                let final_overflow = day_overflow && overflow;
                (self.0.peek().unwrap(), final_overflow)
            } else {
                (self.0.peek().unwrap(), true)
            }
        }

        pub fn next(&mut self, days_overflow: bool) -> (u8, bool) {
            if days_overflow {
                self.0.checked_next().unwrap()
            } else {
                (self.0.peek().unwrap(), false)
            }
        }
    }
}

mod time {
    use crate::schedule::iterator::CopyRing;

    #[derive(Clone, Debug)]
    pub struct Seconds(CopyRing<u8>);

    #[derive(Clone, Debug)]
    pub struct Minutes(CopyRing<u8>);

    #[derive(Clone, Debug)]
    pub struct Hours(CopyRing<u8>);

    impl Seconds {
        pub fn new(copy_ring: CopyRing<u8>) -> Self {
            Self(copy_ring)
        }

        /// Returns the first second that occurs after the given
        /// number of seconds. Rotates the inner buffer so that
        /// calling `next` yields the following value.
        ///
        /// If the inner buffer wrapped back to the earliest second,
        /// then overflow has occurred and the bool is `true`.
        ///
        /// Otherwise, the bool is `false` and no overflow
        /// has occurred.
        pub fn first_after(&mut self, secs: u8) -> (u8, bool) {
            let mut found = false;
            self.0.reset();
            for seconds in self.0.until_start() {
                if seconds >= secs {
                    found = true;
                    break;
                }
            }
            if found {
                self.0.rotate_right(1);
                (self.0.next().unwrap(), false)
            } else {
                (self.0.next().unwrap(), true)
            }
        }

        /// Returns the next second in the inner
        /// buffer, along with whether overflow
        /// occurred. For seconds, overflow
        /// occurs when the seconds passes 59
        /// and wraps back to 0.
        pub fn next(&mut self) -> (u8, bool) {
            self.0.checked_next().unwrap()
        }
    }

    impl Minutes {
        pub fn new(copy_ring: CopyRing<u8>) -> Self {
            Self(copy_ring)
        }

        /// Returns the first minute that occurs after the given
        /// number of minutes. Rotates the inner buffer so that
        /// calling `next` yields the following value.
        ///
        /// If the inner buffer wrapped back to the earliest minute,
        /// then overflow has occurred and the bool is `true`.
        ///
        /// Otherwise, the bool is `false` and no overflow
        /// has occurred.
        pub fn first_after(&mut self, mins: u8, secs_overflow: bool) -> (u8, bool) {
            let mut found = false;
            let mut overflow = false;
            self.0.reset();
            for minutes in self.0.one_cycle().checked() {
                if minutes.0 >= mins {
                    found = true;
                    overflow = minutes.1;
                    break;
                }
                eprintln!("minute: {:?} --- mins: {}", minutes, mins);
            }
            if secs_overflow {
                overflow = self.0.checked_next().unwrap().1 || overflow;
                dbg!(overflow);
            }
            if found {
                self.0.rotate_right(1);
                let final_overflow = secs_overflow && overflow;
                (self.0.peek().unwrap(), final_overflow)
            } else {
                (self.0.peek().unwrap(), true)
            }
        }

        /// Returns the next minute in the inner
        /// buffer, along with whether overflow
        /// occurred. For minutes, overflow
        /// occurs when the minutes passes 59
        /// and wraps back to 0.
        pub fn next(&mut self, secs_overflow: bool) -> (u8, bool) {
            if secs_overflow {
                self.0.checked_next().unwrap()
            } else {
                (self.0.peek().unwrap(), false)
            }
        }
    }

    impl Hours {
        pub fn new(copy_ring: CopyRing<u8>) -> Self {
            Self(copy_ring)
        }

        /// Returns the first hour that occurs after the given
        /// number of hours. Rotates the inner buffer so that
        /// calling `next` yields the following value.
        ///
        /// If the inner buffer wrapped back to the earliest hour,
        /// then overflow has occurred and the bool is `true`.
        ///
        /// Otherwise, the bool is `false` and no overflow
        /// has occurred.
        pub fn first_after(&mut self, hrs: u8, mins_overflow: bool) -> (u8, bool) {
            let mut found = false;
            let mut overflow = false;
            self.0.reset();
            for hour in self.0.one_cycle().checked() {
                if hour.0 >= hrs {
                    found = true;
                    overflow = hour.1;
                    break;
                }
            }
            if mins_overflow {
                overflow = self.0.checked_next().unwrap().1 || overflow
            }
            if found {
                self.0.rotate_right(1);
                let final_overflow = mins_overflow && overflow;
                (self.0.peek().unwrap(), final_overflow)
            } else {
                (self.0.peek().unwrap(), true)
            }
        }

        /// Returns the next hour in the inner
        /// buffer, along with whether overflow
        /// occurred. For hours, overflow
        /// occurs when the hours passes 23
        /// and wraps back to 0.
        pub fn next(&mut self, mins_overflow: bool) -> (u8, bool) {
            if mins_overflow {
                self.0.checked_next().unwrap()
            } else {
                (self.0.peek().unwrap(), false)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Time {
    secs: Seconds,
    mins: Minutes,
    hours: Hours,
}

#[derive(Clone, Debug)]
pub struct Date {
    days: Days,
    months: Months,
    year: u32,
    cache: Cache,
}

#[derive(Clone, Debug, Default)]
struct Cache {
    last_day: u8,
    last_month: u8,
    last_year: u32,
}

#[derive(Default, Debug)]
pub struct TimeBuilder {
    secs: Option<CopyRing<u8>>,
    mins: Option<CopyRing<u8>>,
    hours: Option<CopyRing<u8>>,
}

#[derive(Default, Debug)]
pub struct DateBuilder {
    days_of_the_week: Option<CopyRing<u8>>,
    days_of_the_month: Option<CopyRing<u8>>,
    months: Option<CopyRing<u8>>,
    year: Option<u32>,
}

impl Date {
    pub fn builder() -> DateBuilder {
        Default::default()
    }
}

impl Time {
    pub fn builder() -> TimeBuilder {
        Default::default()
    }
}

impl DateBuilder {
    pub fn with_weekdays(&mut self, weekdays: impl Into<CopyRing<u8>>) -> &mut Self {
        self.days_of_the_week = Some(weekdays.into());
        self
    }

    pub fn with_weekdays_iter(&mut self, weekdays: impl IntoIterator<Item = u8>) -> &mut Self {
        self.days_of_the_week = Some(CopyRing::from_iter(weekdays));
        self
    }

    pub fn with_month_days(&mut self, month_days: impl Into<CopyRing<u8>>) -> &mut Self {
        self.days_of_the_month = Some(month_days.into());
        self
    }

    pub fn with_month_days_iter(&mut self, month_days: impl IntoIterator<Item = u8>) -> &mut Self {
        self.days_of_the_month = Some(CopyRing::from_iter(month_days));
        self
    }

    pub fn with_months_iter(&mut self, months: impl IntoIterator<Item = u8>) -> &mut Self {
        self.months = Some(CopyRing::from_iter(months));
        self
    }

    pub fn with_months(&mut self, months: impl Into<CopyRing<u8>>) -> &mut Self {
        self.months = Some(months.into());
        self
    }

    pub fn with_year(&mut self, year: u32) -> &mut Self {
        self.year = Some(year);
        self
    }

    pub fn build(&mut self) -> Result<Date, Error> {
        let days = match (self.days_of_the_month.take(), self.days_of_the_week.take()) {
            (None, None) => return Err(Error::MissingField),
            (None, Some(week)) => Days::Week((week, DayCache::default())),
            (Some(month), None) => Days::Month(month),
            (Some(month), Some(week)) => Days::Both {
                month,
                week: (week, DayCache::default()),
            },
        };
        let months = self.months.take().ok_or(Error::MissingField)?;
        let year = self.year.take().ok_or(Error::MissingField)?;

        if months.is_empty()
            || match &days {
                Days::Both {
                    month,
                    week: (week, _),
                } => month.is_empty() || week.is_empty(),
                Days::Month(month) => month.is_empty(),
                Days::Week((week, _)) => week.is_empty(),
            }
        {
            return Err(Error::EmptyRing);
        }

        if months.first().unwrap() < 1
            || months.last().unwrap() > 31
            || match &days {
                Days::Both {
                    month,
                    week: (week, _),
                } => {
                    week.last().unwrap() >= 7
                        || month.first().unwrap() < 1
                        || month.last().unwrap() > 31
                }
                Days::Month(month) => month.first().unwrap() < 1 || month.last().unwrap() > 31,
                Days::Week((week, _)) => week.last().unwrap() >= 7,
            }
        {
            return Err(Error::OutOfRange);
        }

        Ok(Date {
            days,
            months: Months::new(months),
            year,
            cache: Default::default(),
        })
    }
}

impl TimeBuilder {
    pub fn with_secs_iter(&mut self, secs: impl IntoIterator<Item = u8>) -> &mut Self {
        self.secs = Some(CopyRing::from_iter(secs));
        self
    }

    pub fn with_secs(&mut self, secs: impl Into<CopyRing<u8>>) -> &mut Self {
        self.secs = Some(secs.into());
        self
    }

    pub fn with_mins_iter(&mut self, mins: impl IntoIterator<Item = u8>) -> &mut Self {
        self.mins = Some(CopyRing::from_iter(mins));
        self
    }

    pub fn with_mins(&mut self, mins: impl Into<CopyRing<u8>>) -> &mut Self {
        self.mins = Some(mins.into());
        self
    }

    pub fn with_hours_iter(&mut self, hours: impl IntoIterator<Item = u8>) -> &mut Self {
        self.hours = Some(CopyRing::from_iter(hours));
        self
    }

    pub fn with_hours(&mut self, hours: impl Into<CopyRing<u8>>) -> &mut Self {
        self.hours = Some(hours.into());
        self
    }

    pub fn build(&mut self) -> Result<Time, Error> {
        if self.secs.is_none() || self.mins.is_none() || self.hours.is_none() {
            return Err(Error::MissingField);
        }

        let secs = self.secs.take().unwrap();
        let mins = self.mins.take().unwrap();
        let hours = self.hours.take().unwrap();

        if secs.is_empty() || mins.is_empty() || hours.is_empty() {
            return Err(Error::EmptyRing);
        }

        if secs.last().unwrap() >= 60 || mins.last().unwrap() >= 60 || hours.last().unwrap() >= 24 {
            return Err(Error::OutOfRange);
        }

        Ok(Time {
            secs: Seconds::new(secs),
            mins: Minutes::new(mins),
            hours: Hours::new(hours),
        })
    }
}
