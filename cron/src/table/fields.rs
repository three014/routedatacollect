use self::{
    date::{DayCache, Days, Months},
    time::{Hours, Minutes, Seconds},
};
use super::{CronRing, Error};
use crate::CopyRing;
use chrono::{NaiveDate, NaiveTime};
use date::LastUsed;

mod date {
    use crate::{table::CronRing, CopyRing};

    #[derive(Clone, Debug)]
    pub enum Days {
        Both {
            month: CronRing,
            week: (CronRing, Option<DayCache>),
        },
        Month(CronRing),
        Week((CronRing, Option<DayCache>)),
    }

    #[derive(Clone, Debug)]
    pub struct Months(CronRing);

    #[derive(Clone, Debug, Default)]
    pub struct Years(Option<u32>);

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
        /// along with the next weekday, from 0-6
        Week(Option<(u8, u8)>),
        Both {
            month: Option<u8>,
            week: Option<(u8, u8)>,
        },
        /// The next day of the month, from 1-31
        Month(Option<u8>),
    }

    impl Days {
        pub fn week(ring: CronRing) -> Self {
            Self::Week((ring, None))
        }

        pub fn month(ring: CronRing) -> Self {
            Self::Month(ring)
        }

        pub fn both(month: CronRing, week: CronRing) -> Self {
            Self::Both {
                month,
                week: (week, None),
            }
        }

        pub fn reset(&mut self) {
            let _ = self.apply_with_both(CopyRing::reset, |(w, _)| w.reset());
        }

        pub(self) const fn num_weekdays_since(start_weekday: i16, end_weekday: i16) -> u8 {
            const DAYS_IN_A_WEEK: i16 = 7;
            ((end_weekday + DAYS_IN_A_WEEK - start_weekday) % DAYS_IN_A_WEEK) as u8
        }

        pub fn cache_mut(&mut self) -> Option<&mut Option<DayCache>> {
            self.apply_with_week(|(_, cache)| cache)
        }

        fn cache(&self) -> Option<&DayCache> {
            self.query_with_week(|(_, cache)| cache)
                .and_then(|cache| cache.as_ref())
        }

        /// Returns the last day saved in the cache, or the last day in the month
        /// copyring if no cache exists.
        pub fn last_day(&self) -> u8 {
            if let Some(cache) = self.cache() {
                cache.last_month_day
            } else {
                self.query_with_month(|month| month.peek_prev().unwrap())
                    .unwrap()
            }
        }

        pub fn query_with_week<'a: 'b, 'b, W, U>(&'a self, week_fn: W) -> Option<U>
        where
            W: FnOnce(&'b (CronRing, Option<DayCache>)) -> U,
        {
            match self {
                Days::Both { month: _, week } => Some(week_fn(week)),
                Days::Week(week) => Some(week_fn(week)),
                _ => None,
            }
        }

        pub fn query_with_month<'a: 'b, 'b, M, U>(&'a self, month_fn: M) -> Option<U>
        where
            M: FnOnce(&'b CronRing) -> U,
        {
            match self {
                Days::Both { month, week: _ } => Some(month_fn(month)),
                Days::Month(month) => Some(month_fn(month)),
                _ => None,
            }
        }

        pub fn query_with_both<'a: 'b, 'b, M, W, T, U>(
            &'a self,
            month_fn: M,
            week_fn: W,
        ) -> (Option<T>, Option<U>)
        where
            M: FnOnce(&'b CronRing) -> T,
            W: FnOnce(&'b (CronRing, Option<DayCache>)) -> U,
        {
            match self {
                Days::Both { month, week } => (Some(month_fn(month)), Some(week_fn(week))),
                Days::Month(month) => (Some(month_fn(month)), None),
                Days::Week(week) => (None, Some(week_fn(week))),
            }
        }

        fn apply_with_week<'a: 'b, 'b, W, U>(&'a mut self, week_fn: W) -> Option<U>
        where
            W: FnOnce(&'b mut (CronRing, Option<DayCache>)) -> U,
        {
            match self {
                Days::Both { month: _, week } => Some(week_fn(week)),
                Days::Week(week) => Some(week_fn(week)),
                _ => None,
            }
        }

        fn apply_with_both<'a: 'b, 'b, M, W, T, U>(
            &'a mut self,
            month_fn: M,
            week_fn: W,
        ) -> (Option<T>, Option<U>)
        where
            M: FnOnce(&'b mut CronRing) -> T,
            W: FnOnce(&'b mut (CronRing, Option<DayCache>)) -> U,
        {
            match self {
                Days::Both { month, week } => (Some(month_fn(month)), Some(week_fn(week))),
                Days::Month(month) => (Some(month_fn(month)), None),
                Days::Week(week) => (None, Some(week_fn(week))),
            }
        }

        pub fn first_after(
            &mut self,
            time_overflow: bool,
            days_month: u8,
            days_week: u8,
            days_in_month: u8,
        ) -> NextDay {
            self.reset();
            let result = self.apply_with_both(
                |month_ring| {
                    month_ring
                        .binary_search_or_greater_by(|probe| {
                            probe.cmp(&(days_month + time_overflow as u8))
                        })
                        .filter(|&(day, overflow)| {
                            !overflow && Self::check_for_end_of_month(day, days_in_month)
                        })
                        .map(|(day, _)| day)
                },
                |(week_ring, _)| {
                    week_ring
                        .binary_search_or_greater_by(|probe| {
                            probe.cmp(&(days_week + time_overflow as u8))
                        })
                        .map(|(day, _)| day)
                        .or_else(|| week_ring.next())
                        .map(|day| {
                            let day_of_month =
                                days_month + Days::num_weekdays_since(days_week.into(), day.into());
                            let day_of_week = day;
                            (day_of_month, day_of_week)
                        })
                        .filter(|&(day, _)| Self::check_for_end_of_month(day, days_in_month))
                },
            );
            match result {
                (None, None) => unreachable!("Days should have one or both of the fields."),
                (None, Some(week)) => NextDay::Week(week),
                (Some(month), None) => NextDay::Month(month),
                (Some(month), Some(week)) => NextDay::Both { month, week },
            }
        }

        pub fn next(&mut self, time_overflow: bool, days_in_month: u8) -> NextDay {
            let (month_overflow, week_overflow) = if let Some(cache) = self.cache() {
                match cache.last_used {
                    LastUsed::Week => (false, time_overflow),
                    LastUsed::Month => (time_overflow, false),
                    LastUsed::Both => (time_overflow, time_overflow),
                }
            } else {
                (time_overflow, false)
            };
            let result = self.apply_with_both(
                |month_ring| {
                    Some(super::next(month_ring, month_overflow))
                        .filter(|&(day, overflow)| {
                            !overflow && Self::check_for_end_of_month(day, days_in_month)
                        })
                        .map(|(day, _)| day)
                },
                |(week_ring, cache)| {
                    Some(super::next(week_ring, week_overflow))
                        .map(|(day, _)| day)
                        .or_else(|| week_ring.next())
                        .map(|day| {
                            let last_month_day = cache.as_ref().unwrap().last_month_day;
                            let last_weekday = cache.as_ref().unwrap().last_weekday;
                            let day_of_month = last_month_day
                                + Days::num_weekdays_since(last_weekday.into(), day.into());
                            let day_of_week = day;
                            (day_of_month, day_of_week)
                        })
                        .filter(|&(day, _)| {
                            Self::check_for_end_of_month(
                                day,
                                cache.as_ref().unwrap().last_month_day,
                            )
                        })
                },
            );
            match result {
                (None, None) => unreachable!("Days should have one or both of the fields."),
                (None, Some(week)) => NextDay::Week(week),
                (Some(month), None) => NextDay::Month(month),
                (Some(month), Some(week)) => NextDay::Both { month, week },
            }
        }

        fn check_for_end_of_month(day: u8, days_in_month: u8) -> bool {
            day <= days_in_month
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
            super::first_after(&mut self.0, false, month)
        }

        pub fn next(&mut self, day_overflow: bool) -> (u8, bool) {
            super::next(&mut self.0, day_overflow)
        }

        pub fn reset(&mut self) {
            self.0.reset()
        }
    }

    #[cfg(test)]
    mod test {
        use crate::table::fields::date::Days;

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

        #[test]
        fn days_between_weekdays_works() {
            let l = 0;
            let r = 1;
            assert_eq!(1, Days::num_weekdays_since(l, r));

            let l = 5;
            let r = 1;
            assert_eq!(3, Days::num_weekdays_since(l, r));

            let l = 2;
            let r = 6;
            assert_eq!(4, Days::num_weekdays_since(l, r));

            let l = 6;
            let r = 5;
            assert_eq!(6, Days::num_weekdays_since(l, r));
        }
    }
}

mod time {
    use crate::table::CronRing;

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
            super::first_after(&mut self.0, false, sec)
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
            super::first_after(&mut self.0, sec_overflow, min)
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
            super::first_after(&mut self.0, min_overflow, hr)
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
        use crate::CopyRing;
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
        (ring.peek_next().unwrap(), false)
    }
}

fn first_after(ring: &mut CronRing, overflow: bool, then: u8) -> (u8, bool) {
    // let found = ring.until_start().find(|&now| cmp_fn(now, then));
    let found = ring
        .binary_search_or_greater_by(|probe| probe.cmp(&(then + overflow as u8)))
        .filter(|&(_, overflow)| !overflow)
        .map(|(next, _)| next);
    if let Some(next) = found {
        (next, false)
    } else {
        (ring.next().unwrap(), true)
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
    fn at_year_limit(&self, starting_year: u32) -> bool {
        if let Some(year) = self.year {
            year - starting_year >= 4
        } else {
            false
        }
    }

    fn set_year(&mut self, year: u32) {
        self.year = Some(year)
    }

    fn year_mut_checked(&mut self) -> Option<&mut u32> {
        self.year.as_mut()
    }

    pub fn first_after(
        &mut self,
        mut time_overflow: bool,
        days_month: u8,
        days_week: u8,
        starting_month: u8,
        starting_year: u32,
    ) -> Option<NaiveDate> {
        self.months.reset();
        let months_to_days_no_leap = crate::MONTH_TO_DAYS_NO_LEAP;
        let mut found = false;
        let mut first_run = true;
        while !found && !self.at_year_limit(starting_year) {
            // Step 1: Set the months to the first available month
            let (month, year_overflow) = if first_run {
                self.months.first_after(starting_month)
            } else {
                self.months.next(true)
            };
            if let Some(year) = self.year_mut_checked() {
                *year += year_overflow as u32;
            } else {
                self.set_year(starting_year + year_overflow as u32);
            }

            // Step 2: If the next month and year are not equal to the given values, then
            //         Set days_month to 1, and calculate the days_week from the
            //         `month`/`days_month`/`self.year` value.
            let mut new_days_month = days_month;
            let mut new_days_week = days_week;
            if month != starting_month || self.year_unchecked() != starting_year {
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
                let months_passed_exclusive = Self::calc_months_passed_exclusive(
                    self.year_unchecked(),
                    month,
                    starting_year,
                    starting_month,
                );

                // Now, we calculate the number of days that have passed from the open
                // range, and we don't account for leap years/days at all.
                let sum_days: u32 = days_of_the_month
                    .take_ref(months_passed_exclusive as usize)
                    .map(Into::<u32>::into)
                    .sum();

                // Then, we calculate the leap years separately, but still only considering
                // the years within the open range. Because of that, this might not enter
                // the loop at all if the years are the same. We'll come back to this later.
                let leap_days = ((starting_year + 1)..self.year_unchecked())
                    .filter(|&year| crate::is_leap_year(year))
                    .count();

                let first_month_days = {
                    let mut days_in_this_month =
                        months_to_days_no_leap[(starting_month - 1) as usize];
                    if crate::is_leap_year(starting_year) && starting_month == 2 {
                        days_in_this_month += 1;
                    }
                    days_in_this_month - days_month + 1
                };
                let days_until_first_of_next_month =
                    sum_days + leap_days as u32 + first_month_days as u32;

                // With the number of days until the first of the month, we can now calculate the
                // weekday, and finally set the days of the month and week.
                new_days_week =
                    Days::next_weekday_from_last(days_week as u32, days_until_first_of_next_month);
                new_days_month = 1;

                // Also, we can set time_overflow to false, regardless of what it was before,
                // because we've advanced the month and year so that any day we find will
                // be past the overflow that the time could've caused.
                time_overflow = false;
            }

            // Step 3: Find the next available day for each type.
            //         The result will be based on which types the
            //         Days struct has.
            let days_in_this_month = {
                let mut days_in_this_month = months_to_days_no_leap[(month - 1) as usize];
                if crate::is_leap_year(self.year_unchecked()) && month == 2 {
                    days_in_this_month += 1;
                }
                days_in_this_month
            };
            let next_day = self.days.first_after(
                time_overflow,
                new_days_month,
                new_days_week,
                days_in_this_month,
            );
            let next_day = match next_day {
                date::NextDay::Week(next_day) => {
                    if let Some((next_of_month, next_of_week)) = next_day {
                        let cache = self.days.cache_mut().unwrap();
                        *cache = Some(DayCache {
                            last_month_day: next_of_month,
                            last_weekday: next_of_week,
                            last_used: LastUsed::Week,
                        });
                        Some(next_of_month)
                    } else {
                        None
                    }
                }
                date::NextDay::Both {
                    week: next_day_week,
                    month: next_day_month,
                } => {
                    let cache = self.days.cache_mut().unwrap();
                    match (next_day_month, next_day_week) {
                        (None, None) => None,
                        (None, Some((next_of_month, next_of_week))) => {
                            *cache = Some(DayCache {
                                last_month_day: next_of_month,
                                last_weekday: next_of_week,
                                last_used: LastUsed::Week,
                            });
                            Some(next_of_month)
                        }
                        (Some(next), None) => {
                            *cache = Some(DayCache {
                                last_month_day: next,
                                last_used: LastUsed::Month,
                                last_weekday: Days::next_weekday_from_last(
                                    new_days_week as u32,
                                    (next - new_days_month) as u32,
                                ),
                            });
                            Some(next)
                        }
                        (Some(next_from_month_ring), Some((next_from_week_ring, next_weekday))) => {
                            match next_from_month_ring.cmp(&next_from_week_ring) {
                                std::cmp::Ordering::Less => {
                                    *cache = Some(DayCache {
                                        last_month_day: next_from_month_ring,
                                        last_used: LastUsed::Month,
                                        last_weekday: Days::next_weekday_from_last(
                                            new_days_week as u32,
                                            (next_from_month_ring - new_days_month) as u32,
                                        ),
                                    });
                                    Some(next_from_month_ring)
                                }
                                std::cmp::Ordering::Equal => {
                                    *cache = Some(DayCache {
                                        last_month_day: next_from_month_ring,
                                        last_weekday: next_weekday,
                                        last_used: LastUsed::Both,
                                    });
                                    Some(next_from_month_ring)
                                }
                                std::cmp::Ordering::Greater => {
                                    *cache = Some(DayCache {
                                        last_month_day: next_from_week_ring,
                                        last_weekday: next_weekday,
                                        last_used: LastUsed::Week,
                                    });
                                    Some(next_from_week_ring)
                                }
                            }
                        }
                    }
                }
                date::NextDay::Month(next_day) => next_day,
            };
            if let Some(next) = next_day {
                self.cache.last_day = next;
                self.cache.last_month = month;
                self.cache.last_year = self.year_unchecked();
                found = true;
            } else {
                first_run = false;
            }
        }

        NaiveDate::from_ymd_opt(
            self.cache.last_year as i32,
            self.cache.last_month as u32,
            self.cache.last_day as u32,
        )
    }

    fn calc_months_passed_exclusive(
        year: u32,
        month: u8,
        starting_year: u32,
        starting_month: u8,
    ) -> u32 {
        let months_in_a_year: u32 = 12;
        let mut end_year = year;
        let end_month = month.checked_sub(2).map(|x| x + 1).unwrap_or_else(|| {
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

        end_year
            .checked_sub(starting_year)
            .and_then(|sub_years| {
                (end_month + months_in_a_year * sub_years).checked_sub(starting_month)
            })
            .unwrap_or_default()
    }

    pub fn next(&mut self) -> Option<NaiveDate> {
        // First, grab the current day, month, and year from our cache
        // We can get the weekday from our day cache, if necessary.

        let days_month = self.cache.last_day;
        let month = self.cache.last_month;
        let year = self.cache.last_year;

        // Note: we might not even need the cache. If we can get
        // the next dates from each of the Date struct's fields,
        // then we don't need to store them in this separate location.
        todo!()
    }

    /// Returns the current year.
    ///
    /// # Panic
    /// Will panic if the year hasn't been set.
    pub fn year_unchecked(&self) -> u32 {
        self.year.unwrap()
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
            (None, Some(week)) => Days::week(week),
            (Some(month), None) => Days::month(month),
            (Some(month), Some(week)) => Days::both(month, week),
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
            || Some(days.query_with_both(
                |m| m.first().unwrap() < 1 || m.last().unwrap() > 31,
                |(w, _)| w.last().unwrap() >= 7,
            ))
            .map(|(m, w)| m.into_iter().chain(w.into_iter()).any(|x| x))
            .unwrap()
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

#[cfg(test)]
mod test {
    use super::{
        date::{Days, Months},
        Date,
    };
    use crate::table::CronRing;
    use chrono::Utc;
    use std::sync::Arc;

    #[test]
    fn first_after_for_leap_day() {
        let mut d = Date {
            days: Days::Month(CronRing::owned([29])),
            months: Months::new(CronRing::owned([2])),
            year: None,
            cache: Default::default(),
        };

        let then = Utc::now();
        let f = d.first_after(false, 24, 6, 6, 2024);
        let now = Utc::now();
        println!("{:?}", now - then);
        if let Some(f) = f {
            dbg!(f);
        } else {
            dbg!("Couldn't get next date");
        }
    }

    #[test]
    fn weekdays_works() {
        let mut d = Date {
            days: Days::Both {
                month: CronRing::arc_with_size(Arc::new([
                    6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 29,
                    30,
                ])),
                week: (
                    CronRing::arc_with_size(Arc::new([2, 5])),
                    Default::default(),
                ),
            },
            months: Months::new(CronRing::borrowed_with_size(&crate::DEFAULT_MONTHS)),
            year: None,
            cache: Default::default(),
        };

        let f = d.first_after(true, 30, 6, 12, 2023);
        if let Some(f) = f {
            dbg!(f);
        } else {
            dbg!("Couldn't get next date");
        }
        dbg!(d);
    }

    #[test]
    fn weekdays_works_for_june() {
        let mut d = Date {
            days: Days::Both {
                month: CronRing::arc_with_size(Arc::new([
                    6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 27,
                    30,
                ])),
                week: (
                    CronRing::arc_with_size(Arc::new([3, 5])),
                    Default::default(),
                ),
            },
            months: Months::new(CronRing::borrowed_with_size(&crate::DEFAULT_MONTHS)),
            year: None,
            cache: Default::default(),
        };

        let f = d.first_after(true, 25, 0, 6, 2023);
        if let Some(f) = f {
            dbg!(f);
        } else {
            dbg!("Couldn't get next date");
        }
        dbg!(d);
    }

    #[test]
    fn first_after_for_dec_jan() {
        let mut d = Date {
            days: Days::Month(CronRing::arc_with_size(Arc::new([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24, 25, 26, 27, 28, 29, 30,
            ]))),
            months: Months::new(CronRing::borrowed_with_size(&crate::DEFAULT_MONTHS)),
            year: None,
            cache: Default::default(),
        };

        let f = d.first_after(true, 30, 6, 12, 2024);
        if let Some(f) = f {
            dbg!(f);
        } else {
            dbg!("Couldn't get next date");
        }
    }

    #[test]
    fn profile() {
        let mut d = Date {
            days: Days::Month(CronRing::arc_with_size(Arc::new([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24, 25, 26, 27, 28, 29, 30,
            ]))),
            months: Months::new(CronRing::borrowed_with_size(&crate::DEFAULT_MONTHS)),
            year: None,
            cache: Default::default(),
        };
        let then = Utc::now();
        for _ in 0..1000000 {
            let a = d.first_after(true, 30, 6, 12, 2024);
        }
        let now = Utc::now();
        println!("{:?}", now - then);
    }
}
