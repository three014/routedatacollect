use self::{
    date::{DayCache, Days, Months},
    time::{Hours, Minutes, Seconds},
};
use super::{CronRing, Error};
use crate::schedule::{iterator::CopyRing, table::fields::date::LastUsed};
use chrono::{NaiveDate, NaiveTime};

mod date {
    use crate::schedule::table::CronRing;

    #[derive(Clone, Debug)]
    pub enum Days {
        Both {
            month: CronRing,
            week: (CronRing, DayCache),
        },
        Month(CronRing),
        Week((CronRing, DayCache)),
    }

    #[derive(Clone, Debug)]
    pub struct Months(CronRing);

    #[derive(Clone, Debug, Default)]
    pub struct DayCache {
        pub last_month_day: u8,
        pub last_weekday: u8,
        pub last_used: LastUsed,
    }

    #[derive(Clone, Copy, Debug, Default)]
    pub enum LastUsed {
        #[default]
        Week,
        Month,
        Both,
    }

    #[derive(Clone, Debug)]
    pub enum NextDay {
        /// The next day of the month, from 1-31
        Week(Option<u8>),
        Both {
            week: Option<u8>,
            month: Option<u8>,
        },
        /// The next day of the month, from 1-31
        Month(Option<u8>),
    }

    pub mod days {
        use super::DayCache;
        use crate::schedule::table::CronRing;

        #[derive(Debug)]
        pub struct Both<'a> {
            pub month: &'a mut CronRing,
            pub week: &'a mut (CronRing, DayCache),
        }
        #[derive(Debug)]
        pub struct Month<'a>(pub &'a mut CronRing);
        #[derive(Debug)]
        pub struct Week<'a>(pub &'a mut (CronRing, DayCache));
    }

    impl DayCache {
        pub fn set(&mut self, new_summary: DayCache) {
            self.last_month_day = new_summary.last_month_day;
            self.last_used = new_summary.last_used;
            self.last_weekday = new_summary.last_weekday;
        }
    }

    impl Days {
        pub fn reset(&mut self) {
            match self {
                Days::Both {
                    month,
                    week: (week, _),
                } => {
                    month.reset();
                    week.reset();
                }
                Days::Month(month) => month.reset(),
                Days::Week((week, _)) => week.reset(),
            }
        }

        pub fn both(&mut self) -> Option<days::Both> {
            match self {
                Days::Both { month, week } => Some(days::Both { month, week }),
                _ => None,
            }
        }

        pub fn week(&mut self) -> Option<days::Week> {
            match self {
                Days::Week(week) => Some(days::Week(week)),
                _ => None,
            }
        }

        pub fn month(&mut self) -> Option<days::Month> {
            match self {
                Days::Month(month) => Some(days::Month(month)),
                _ => None,
            }
        }

        pub const fn num_weekdays_since(start_weekday: i16, end_weekday: i16) -> u8 {
            todo!()
        }

        pub const fn next_weekday_from_last(first_weekday: u32, num_days_to_advance: u32) -> u8 {
            let days_in_a_week = 7;
            let result = (first_weekday + num_days_to_advance) % days_in_a_week;
            result as u8
        }
    }

    impl Months {
        pub const fn new(copy_ring: CronRing) -> Self {
            Self(copy_ring)
        }

        pub fn first_after(&mut self, month: u8) -> (u8, bool) {
            self.0.reset();
            let found = self.0.until_start().find(|&our_month| our_month >= month);
            if let Some(next) = found {
                (next, false)
            } else {
                (self.0.next().unwrap(), true)
            }
        }

        pub fn next(&mut self, day_overflow: bool) -> (u8, bool) {
            super::next(&mut self.0, day_overflow)
        }
    }

    #[cfg(test)]
    mod test {
        use crate::schedule::table::fields::date::Days;

        #[test]
        fn next_weekday_from_last_works() {
            let start = 0;
            let n = 7;
            assert_eq!(0, Days::next_weekday_from_last(start, n));

            let start = 1;
            let n = 13;
            assert_eq!(0, Days::next_weekday_from_last(start, n));

            let start = 5;
            let n = 0;
            assert_eq!(5, Days::next_weekday_from_last(start, n));
        }
    }
}

mod time {
    use crate::schedule::table::CronRing;

    #[derive(Clone, Debug)]
    pub struct Seconds(CronRing);

    #[derive(Clone, Debug)]
    pub struct Minutes(CronRing);

    #[derive(Clone, Debug)]
    pub struct Hours(CronRing);

    impl Seconds {
        pub const fn new(copy_ring: CronRing) -> Self {
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
        pub fn first_after(&mut self, sec: u8) -> (u8, bool) {
            self.0.reset();
            let found = self.0.until_start().find(|&second| second >= sec);
            if let Some(next) = found {
                (next, false)
            } else {
                (self.0.next().unwrap(), true) // We know that overflow occurred
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
        pub const fn new(copy_ring: CronRing) -> Self {
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
        pub fn first_after(&mut self, min: u8, sec_overflow: bool) -> (u8, bool) {
            self.0.reset();
            let found = self.0.until_start().find(|&minute| minute >= min);
            if let Some(next) = found {
                if sec_overflow {
                    self.0.checked_next().unwrap()
                } else {
                    (next, false)
                }
            } else {
                // We know that overflow occurred, so the next
                // minute will be in the next hour, which will be much
                // later than the origin time. Therefore,
                // no need to pay attention to `sec_overflow` in
                // this case.
                (self.0.next().unwrap(), true)
            }
        }

        /// Returns the next minute in the inner
        /// buffer, along with whether overflow
        /// occurred. For minutes, overflow
        /// occurs when the minutes passes 59
        /// and wraps back to 0.
        pub fn next(&mut self, sec_overflow: bool) -> (u8, bool) {
            super::next(&mut self.0, sec_overflow)
        }
    }

    impl Hours {
        pub const fn new(copy_ring: CronRing) -> Self {
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
        pub fn first_after(&mut self, hr: u8, min_overflow: bool) -> (u8, bool) {
            self.0.reset();
            let found = self.0.until_start().find(|&hour| hour >= hr);
            if let Some(next) = found {
                if min_overflow {
                    self.0.checked_next().unwrap()
                } else {
                    (next, false)
                }
            } else {
                (self.0.next().unwrap(), true)
            }
        }

        /// Returns the next hour in the inner
        /// buffer, along with whether overflow
        /// occurred. For hours, overflow
        /// occurs when the hours passes 23
        /// and wraps back to 0.
        pub fn next(&mut self, min_overflow: bool) -> (u8, bool) {
            super::next(&mut self.0, min_overflow)
        }
    }

    #[cfg(test)]
    mod test {
        use super::{Hours, Minutes, Seconds};
        use crate::schedule::iterator::CopyRing;
        use chrono::{Timelike, Utc};
        use rand::Rng;

        const THRESHOLD: i32 = 50;
        const UPPER: i32 = 100;

        fn gen_range_mins_or_secs() -> Vec<u8> {
            let mut v = vec![];
            let mut rng = rand::thread_rng();
            for i in 0u8..60 {
                if rng.gen::<i32>() % UPPER > THRESHOLD {
                    v.push(i)
                }
            }
            if v.is_empty() {
                v.push(rng.gen::<u8>() % 60)
            }
            v
        }

        fn gen_range_hours() -> Vec<u8> {
            let mut v = vec![];
            let mut rng = rand::thread_rng();
            for i in 0u8..24 {
                if rng.gen::<i32>() % UPPER > THRESHOLD {
                    v.push(i)
                }
            }
            if v.is_empty() {
                v.push(rng.gen::<u8>() % 24)
            }
            v
        }

        #[test]
        fn first_after_works_for_secs() {
            let mut seconds =
                Seconds::new(CopyRing::arc_with_size(gen_range_mins_or_secs().into()));
            let now = Utc::now();

            let next = seconds.first_after(now.second() as u8);
            match next.1 {
                true => assert!((next.0 as u32) < now.second()),
                false => assert!((next.0 as u32) >= now.second()),
            }
        }

        #[test]
        fn next_for_seconds() {
            let mut secs = Seconds::new(CopyRing::arc_with_size(gen_range_mins_or_secs().into()));
            let mut rng = rand::thread_rng();
            let s = rng.gen::<u8>() % 60;
            let first = secs.first_after(s);
            eprintln!("First after {} seconds: {:?}", s, first);
            dbg!(secs.next());
        }

        #[test]
        fn first_after_works_for_mins_no_overflow() {
            let mut minutes =
                Minutes::new(CopyRing::arc_with_size(gen_range_mins_or_secs().into()));
            let now = Utc::now();

            let next = minutes.first_after(now.minute() as u8, false);
            match next.1 {
                true => assert!((next.0 as u32) < now.minute()),
                false => assert!((next.0 as u32) >= now.minute()),
            }
        }

        #[test]
        fn first_after_works_for_mins_overflow() {
            let mut minutes =
                Minutes::new(CopyRing::arc_with_size(gen_range_mins_or_secs().into()));
            for i in 0..60 {
                let now2 = i;

                let next = minutes.first_after(now2, true);
                //eprintln!("now: {} minutes", now2);
                //dbg!(next);
                //dbg!(&minutes);
                match next.1 {
                    true => assert!((next.0) < now2),
                    false => assert!((next.0) >= now2),
                }
            }
        }

        #[test]
        fn first_after_works_for_hours_overflow() {
            let mut hours = Hours::new(CopyRing::arc_with_size(gen_range_hours().into()));
            for i in 0..24 {
                let now2 = i;

                let next = hours.first_after(now2, true);
                //eprintln!("now: {} hours", now2);
                //dbg!(next);
                //dbg!(&hours);
                match next.1 {
                    true => assert!((next.0) < now2),
                    false => assert!((next.0) >= now2),
                }
            }
        }
    }
}

fn next(ring: &mut CronRing, overflow: bool) -> (u8, bool) {
    if overflow {
        ring.checked_next().unwrap()
    } else {
        (ring.peek().unwrap(), false)
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
    year: Option<u32>,
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
    secs: Option<CronRing>,
    mins: Option<CronRing>,
    hours: Option<CronRing>,
}

#[derive(Default, Debug)]
pub struct DateBuilder {
    days_week: Option<CronRing>,
    days_month: Option<CronRing>,
    months: Option<CronRing>,
}

impl Date {
    pub fn first_after(
        &mut self,
        mut time_overflow: bool,
        mut days_month: u8,
        mut days_week: u8,
        mut starting_month: u8,
        mut starting_year: u32,
    ) -> Option<NaiveDate> {
        let months_to_days_no_leap = crate::MONTH_TO_DAYS_NO_LEAP;
        let mut found = false;
        while !found
            && if let Some(year) = self.year {
                year - starting_year < 4
            } else {
                true
            }
        {
            // Check for next day, only until the overflow occurs
            // (or until day is found).
            //
            //     Move both days and weeks with
            //     `CopyRing::until_start()` iter, keeping
            //     track of the current value outside the
            //     loop.
            //
            //     If not found for either type of day, then
            //     advance the month to the next month and repeat
            //     first loop.
            //
            //         If the month overflowed, increment years by 1.
            //         After advancing the month, recalcuate the
            //         given weekday so that on the next run, the
            //         weekday ring only has to find the
            //         first weekday after the first day of the new
            //         month.
            //
            //     If one type found the next day, then check if
            //     that day is actually in the current month.
            //
            //         If not, then reset that type, advance the month
            //         to the next month, then check if the other
            //         type is valid.
            //
            //             If the month overflowed, increment years by 1.
            //
            //

            // Step 1: Set the months to the first available month
            let (month, year_overflow) = self.months.first_after(starting_month);
            self.year = Some(starting_year + year_overflow as u32);

            // Step 2: If the next month and year are not equal to the given values, then
            //         Set days_month to 1, and calculate the days_week from the
            //         `month`/`days_month`/`self.year` value.
            if month != starting_month || self.year.unwrap() != starting_year {
                let first_of_the_month = 1;
                // Calculate the number of days that have passed from the
                // last day/month/year.
                let mut days_of_the_month = CopyRing::borrowed(&months_to_days_no_leap);
                days_of_the_month.rotate_left(starting_month.into()); // Puts us on the month after, not the current month.

                // We want to calculate the number of days from the open range of the first month
                // to the last month. But, we have to account for the fact that the first month and
                // the last month may not be full days, AND that there might be leap days within this
                // entire bound.

                // To make this work, we first calculate the number of months that have passed,
                // not including the first and the last month. If the first and last month are the
                // same, then this value should be 0. Furthermore, if the first and last month are
                // only a month apart, then this value should also be 0.
                let months_in_a_year: u32 = 12;
                let months_passed_open_range = {
                    let mut end_year = self.year.unwrap();
                    let end_month = month
                        .checked_sub(2)
                        .and_then(|x| Some(x + 1))
                        .unwrap_or_else(|| {
                            end_year -= 1;
                            months_in_a_year as u8
                        }) as u32;

                    let mut starting_year = starting_year;
                    let starting_month: u32 = {
                        let temp_month = starting_month as u32 + 1;
                        if temp_month > months_in_a_year {
                            starting_year += 1;
                            1
                        } else {
                            temp_month
                        }
                    };

                    end_month + months_in_a_year * (end_year - starting_year) - starting_month
                };

                // Now, we calculate the number of days that have passed from the open
                // range, and we don't account for leap years/days at all.
                let sum_days_open_range: u32 = days_of_the_month
                    .take_ref(months_passed_open_range as usize)
                    .map(Into::<u32>::into)
                    .sum();

                // Then, we calculate the leap years separately, but still only considering
                // the years within the open range. Because of that, this might not enter
                // the loop at all if the years are the same. We'll come back to this later.
                let leap_days_open_range = ((starting_year + 1)..self.year.unwrap())
                    .filter(|&year| crate::is_leap_year(year))
                    .count();

                let first_month_days = {
                    let mut days_in_this_month =
                        months_to_days_no_leap[(starting_month - 1) as usize];
                    if crate::is_leap_year(starting_year) && starting_month == 2 {
                        days_in_this_month += 1;
                    }
                    days_in_this_month - days_month
                };
                let days_until_first_of_next_month =
                    sum_days_open_range + leap_days_open_range as u32 + first_month_days as u32;

                // With the number of days until the first of the month, we can now calculate the
                // weekday, and finally set the days of the month and week.
                days_week =
                    Days::next_weekday_from_last(days_week as u32, days_until_first_of_next_month);
                days_month = first_of_the_month;

                // Also, we can set time_overflow to false, regardless of what it was before,
                // because we've advanced the month and year so that any day we find will
                // be past the overflow that the time could've caused.
                time_overflow = false;
            }

            // Step 3: Find the next available day for each type.
            //         The result will be based on which types the
            //         Days struct has.
            let cmp_days = if time_overflow {
                |day, start| day > start
            } else {
                |day, start| day >= start
            };

            let days_in_this_month = {
                let mut days_in_this_month = months_to_days_no_leap[(month - 1) as usize];
                if crate::is_leap_year(self.year.unwrap()) && month == 2 {
                    days_in_this_month += 1;
                }
                days_in_this_month
            };
            let check_for_end_of_month = |day: &u8| *day <= days_in_this_month;

            self.days.reset();
            let next_day = match &mut self.days {
                Days::Both {
                    month,
                    week: (week, cache),
                } => date::NextDay::Both {
                    week: week
                        .until_start()
                        .find(|&day| cmp_days(day, days_week))
                        .or_else(|| week.next())
                        .map(|day| {
                            cache.last_weekday = day; // SAVE THIS VALUE
                            days_month + Days::num_weekdays_since(days_week.into(), day.into())
                        }) // This will always be Some
                        .filter(check_for_end_of_month), // Now it can vary
                    month: month
                        .until_start()
                        .find(|&day| cmp_days(day, days_month))
                        .filter(check_for_end_of_month),
                },
                Days::Month(month) => date::NextDay::Month(
                    month
                        .until_start()
                        .find(|&day| cmp_days(day, days_month))
                        .filter(check_for_end_of_month),
                ),
                Days::Week((week, cache)) => date::NextDay::Week(
                    week.until_start()
                        .find(|&day| cmp_days(day, days_week))
                        .or_else(|| week.next())
                        .map(|day| {
                            cache.last_weekday = day; // SAVE THIS VALUE
                            days_month + Days::num_weekdays_since(days_week.into(), day.into())
                        }) // This will always be Some
                        .filter(check_for_end_of_month), // Now it can vary
                ),
            };

            found = match next_day {
                date::NextDay::Week(weekday) => todo!(),
                date::NextDay::Both {
                    week: next_day_week,
                    month: next_day_month,
                } => {
                    let date::days::Both {
                        month: month_ring,
                        week: (week, cache),
                    } = self.days.both().unwrap();
                    match (next_day_month, next_day_week) {
                        (None, None) => {
                            let (next, overflow) = self.months.next(true);
                            starting_month = next;
                            starting_year = self.year.unwrap() + overflow as u32;
                            false
                        }
                        (None, Some(next)) => {
                            month_ring.rotate_right(1);
                            cache.last_month_day = next;
                            cache.last_used = LastUsed::Week;
                            self.cache.last_day = next;
                            self.cache.last_month = month;
                            self.cache.last_year = self.year.unwrap();
                            true
                        }
                        (Some(next), None) => {
                            week.rotate_right(1);
                            cache.last_month_day = next;
                            cache.last_weekday = Days::next_weekday_from_last(
                                days_week as u32,
                                (next - days_month) as u32,
                            );
                            self.cache.last_day = next;
                            self.cache.last_month = month;
                            self.cache.last_year = self.year.unwrap();
                            true
                        }
                        (Some(next_day_month), Some(next_day_week)) => {
                            match next_day_month.cmp(&next_day_week) {
                                std::cmp::Ordering::Less => todo!(),
                                std::cmp::Ordering::Equal => todo!(),
                                std::cmp::Ordering::Greater => todo!(),
                            }
                        }
                    }
                }
                date::NextDay::Month(next_day) => {
                    if let Some(next) = next_day {
                        self.cache.last_day = next;
                        self.cache.last_month = month;
                        self.cache.last_year = self.year.unwrap();
                        true
                    } else {
                        let (next, overflow) = self.months.next(true);
                        starting_month = next;
                        starting_year = self.year.unwrap() + overflow as u32;
                        false
                    }
                }
            }
        }
        todo!()
    }

    pub fn next(&mut self) -> Option<NaiveDate> {
        todo!()
    }

    const fn calculate_weekday_from(
        start_month: u8,
        start_year: u32,
        start_weekday: u8,
        start_monthday: u8,
        later_month: u8,
        later_year: u32,
        later_monthday: u8,
    ) -> u8 {
        todo!()
    }

    pub fn builder() -> DateBuilder {
        Default::default()
    }
}

impl Time {
    pub fn first_after(&mut self, sec: u8, min: u8, hour: u8) -> Option<(NaiveTime, bool)> {
        let (sec, overflow) = self.secs.first_after(sec);
        let (min, overflow) = self.mins.first_after(min, overflow);
        let (hour, overflow) = self.hours.first_after(hour, overflow);
        let time = NaiveTime::from_hms_opt(hour as u32, min as u32, sec as u32)?;
        Some((time, overflow))
    }

    pub fn next(&mut self) -> Option<(NaiveTime, bool)> {
        let (sec, overflow) = self.secs.next();
        let (min, overflow) = self.mins.next(overflow);
        let (hour, overflow) = self.hours.next(overflow);
        let time = NaiveTime::from_hms_opt(hour as u32, min as u32, sec as u32)?;
        Some((time, overflow))
    }

    pub fn builder() -> TimeBuilder {
        Default::default()
    }
}

impl DateBuilder {
    pub fn with_days_week(&mut self, weekdays: CronRing) -> &mut Self {
        self.days_week = Some(weekdays);
        self
    }

    pub fn with_days_week_iter(&mut self, weekdays: impl IntoIterator<Item = u8>) -> &mut Self {
        self.days_week = Some(CopyRing::arc_with_size(weekdays.into_iter().collect()));
        self
    }

    pub fn with_days_month(&mut self, month_days: CronRing) -> &mut Self {
        self.days_month = Some(month_days);
        self
    }

    pub fn with_days_month_iter(&mut self, month_days: impl IntoIterator<Item = u8>) -> &mut Self {
        self.days_month = Some(CopyRing::arc_with_size(month_days.into_iter().collect()));
        self
    }

    pub fn with_months_iter(&mut self, months: impl IntoIterator<Item = u8>) -> &mut Self {
        self.months = Some(CopyRing::arc_with_size(months.into_iter().collect()));
        self
    }

    pub fn with_months(&mut self, months: CronRing) -> &mut Self {
        self.months = Some(months);
        self
    }

    pub fn build(&mut self) -> Result<Date, Error> {
        let days = match (self.days_month.take(), self.days_week.take()) {
            (None, None) => return Err(Error::MissingField),
            (None, Some(week)) => Days::Week((week, DayCache::default())),
            (Some(month), None) => Days::Month(month),
            (Some(month), Some(week)) => Days::Both {
                month,
                week: (week, DayCache::default()),
            },
        };
        let months = self.months.take().ok_or(Error::MissingField)?;

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
            year: None,
            cache: Default::default(),
        })
    }
}

impl TimeBuilder {
    pub fn with_secs_iter(&mut self, secs: impl IntoIterator<Item = u8>) -> &mut Self {
        self.secs = Some(CopyRing::arc_with_size(secs.into_iter().collect()));
        self
    }

    pub fn with_secs(&mut self, secs: CronRing) -> &mut Self {
        self.secs = Some(secs);
        self
    }

    pub fn with_mins_iter(&mut self, mins: impl IntoIterator<Item = u8>) -> &mut Self {
        self.mins = Some(CopyRing::arc_with_size(mins.into_iter().collect()));
        self
    }

    pub fn with_mins(&mut self, mins: CronRing) -> &mut Self {
        self.mins = Some(mins);
        self
    }

    pub fn with_hours_iter(&mut self, hours: impl IntoIterator<Item = u8>) -> &mut Self {
        self.hours = Some(CopyRing::arc_with_size(hours.into_iter().collect()));
        self
    }

    pub fn with_hours(&mut self, hours: CronRing) -> &mut Self {
        self.hours = Some(hours);
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
