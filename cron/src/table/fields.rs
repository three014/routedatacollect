use self::{
    date::{DayCache, Days, DaysInner, Months},
    time::{Hours, Minutes, Seconds},
};
use super::{CronRing, Error};
use crate::CopyRing;
use chrono::{NaiveDate, NaiveTime};
use date::LastUsed;

mod date;
mod time;

fn next(ring: &mut CronRing, overflow: bool) -> (u8, bool) {
    if overflow {
        ring.checked_next().unwrap()
    } else {
        (ring.peek_prev().unwrap(), false)
    }
}

fn first_after(ring: &mut CronRing, overflow: bool, then: u8) -> (u8, bool) {
    // let found = ring.until_start().find(|&now| cmp_fn(now, then));
    let found = ring
        .binary_search_or_greater(&(then + overflow as u8))
        .filter(|&(_, overflow)| !overflow)
        .map(|(next, _)| next);
    if let Some(next) = found {
        (next, false)
    } else {
        ring.reset();
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
}

#[derive(Clone, Debug, Default)]
struct Cache {
    day: u8,
    month: u8,
    year: u32,
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
        start_days_month: u8,
        start_days_week: u8,
        start_month: u8,
        start_year: u32,
    ) -> Option<NaiveDate> {
        self.months.reset();
        let mut first_run = true;
        let mut date = None;
        let mut days_of_the_month = CopyRing::owned(crate::MONTH_TO_DAYS_NO_LEAP);
        while date.is_none() && !self.at_year_limit(start_year) {
            // Step 1: Set the months to the first available month
            let (month, year_overflow) = if first_run {
                self.months.first_after(start_month)
            } else {
                self.months.next()
            };
            if let Some(year) = self.year_mut_checked() {
                *year += year_overflow as u32;
            } else {
                self.set_year(start_year + year_overflow as u32);
            }

            // Step 2: If the next month and year are not equal to the given values, then
            //         Set days_month to 1, and calculate the days_week from the
            //         `month`/`days_month`/`self.year` value.
            let mut start_days_month = start_days_month;
            let mut start_days_week = start_days_week;
            if month != start_month || self.year_unchecked() != start_year {
                // Calculate the number of days that have passed from the
                // last day/month/year.
                days_of_the_month.reset();
                days_of_the_month.rotate_left(start_month.into()); // Puts us on the month after, not the current month.

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
                    start_year,
                    start_month,
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
                let leap_days = ((start_year + 1)..self.year_unchecked())
                    .filter(|&year| crate::is_leap_year(year))
                    .count();

                let first_month_days = {
                    let days_in_this_month = Self::days_in_month(start_month, start_year);
                    days_in_this_month - start_days_month + 1
                };
                let days_until_first_of_next_month =
                    sum_days + leap_days as u32 + first_month_days as u32;

                // With the number of days until the first of the month, we can now calculate the
                // weekday, and finally set the days of the month and week.
                start_days_week = DaysInner::next_weekday_from_last(
                    start_days_week as u32,
                    days_until_first_of_next_month,
                );
                start_days_month = 1;

                // Also, we can set time_overflow to false, regardless of what it was before,
                // because we've advanced the month and year so that any day we find will
                // be past the overflow that the time could've caused.
                time_overflow = false;
            }

            // Step 3: Find the next available day for each type.
            //         The result will be based on which types the
            //         Days struct has.
            let next_day = {
                let year = self.year_unchecked();
                self.days.first_after(
                    time_overflow,
                    start_days_month,
                    start_days_week,
                    month,
                    year,
                )
            };

            if let Some(next) = next_day {
                date = Some(Cache {
                    day: next,
                    month,
                    year: self.year_unchecked(),
                });
            } else {
                first_run = false;
            };
        }

        date.and_then(|d| NaiveDate::from_ymd_opt(d.year as i32, d.month as u32, d.day as u32))
    }

    pub(self) fn days_in_month(month: u8, year: u32) -> u8 {
        let months_to_days_no_leap = crate::MONTH_TO_DAYS_NO_LEAP;
        let mut days_in_this_month = months_to_days_no_leap[(month - 1) as usize];
        if month == 2 && crate::is_leap_year(year) {
            days_in_this_month += 1;
        }
        days_in_this_month
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

    /// # Panic
    /// Will panic if called before `Date::first_after`, since the
    /// `year` will not have been set, and this function uses that
    /// field.
    pub fn next(&mut self, time_overflow: bool) -> Option<NaiveDate> {
        // The actual first: Check if there's overflow.
        // If there isn't, then we just return the current date again,
        // No calculations necessary.
        let cache = Cache {
            day: self.days.last(),
            month: self.months.last(),
            year: self.year_unchecked(),
        };
        let result = if time_overflow {
            let mut date = None;
            while date.is_none() && !self.at_year_limit(cache.year) {
                // Get the next day (and weekday!)

                // Check if those days are within bounds.
                // If both days are not, then we need
                // to advance the month ring, then
                // recalculate the next weekday and
                // use the first_after function like
                // we did last time.

                // If only the weekday was outta bounds,
                // then we calculate that weekday's month-day,
                // month, and year. But also, in this case
                // the month-ring wins, and we go with that
                // date.
                // However, in this case, we gotta look at
                // the next time we calculate the dates.
                //
                // Next time, if we need to calculate
                // the next weekday because of
                // out-of-bounds-ness, then we need
                // to use the cached data from our
                // weekday's last day.
                // That way, we actually shouldn't have
                // to calculate the weekday for the
                // month-ring at all.
                //
                // [7/2] I'm going to try implementing
                // this for the first_after function
                // first, then try it here if it works.

                let next_day = self.days.next(Self::days_in_month(cache.month, cache.year));
            }
            date
        } else {
            Some(cache)
        };

        result.and_then(|date| {
            NaiveDate::from_ymd_opt(date.year as i32, date.month as u32, date.day as u32)
        })
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

        if months.is_empty() || days.query(CopyRing::is_empty, |(w, _)| w.is_empty()).any() {
            return Err(Error::EmptyRing);
        }

        if months.first().unwrap() < 1
            || months.last().unwrap() > 31
            || days
                .query(
                    |m| m.first().unwrap() < 1 || m.last().unwrap() > 31,
                    |(w, _)| w.last().unwrap() >= 7,
                )
                .any()
        {
            return Err(Error::OutOfRange);
        }

        Ok(Date {
            days,
            months: Months::new(months),
            year: None,
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
    use super::{date::Months, Date};
    use crate::table::{fields::date::Days, CronRing};
    use chrono::Utc;
    use std::sync::Arc;

    #[test]
    fn first_after_for_leap_day() {
        let mut d = Date {
            days: Days::month(CronRing::owned([29])),
            months: Months::new(CronRing::owned([2])),
            year: None,
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
            days: Days::both(
                CronRing::arc_with_size(Arc::new([
                    6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 29,
                    30,
                ])),
                CronRing::arc_with_size(Arc::new([3, 5])),
            ),
            months: Months::new(CronRing::borrowed_with_size(&crate::DEFAULT_MONTHS)),
            year: None,
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
            days: Days::both(
                CronRing::arc_with_size(Arc::new([
                    6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 27,
                    30,
                ])),
                CronRing::arc_with_size(Arc::new([3, 5])),
            ),
            months: Months::new(CronRing::borrowed_with_size(&crate::DEFAULT_MONTHS)),
            year: None,
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
            days: Days::month(CronRing::arc_with_size(Arc::new([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24, 25, 26, 27, 28, 29, 30,
            ]))),
            months: Months::new(CronRing::borrowed_with_size(&crate::DEFAULT_MONTHS)),
            year: None,
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
            days: Days::month(CronRing::arc_with_size(Arc::new([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24, 25, 26, 27, 28, 29, 30,
            ]))),
            months: Months::new(CronRing::borrowed_with_size(&crate::DEFAULT_MONTHS)),
            year: None,
        };
        let then = Utc::now();
        for _ in 0..1000000 {
            let _a = d.first_after(true, 30, 6, 12, 2024);
        }
        let now = Utc::now();
        println!("Diff: {:?}", now - then);
    }
}
