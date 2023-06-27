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
        (ring.peek_next().unwrap(), false)
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
        let mut result = None;
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
                new_days_week = DaysInner::next_weekday_from_last(
                    days_week as u32,
                    days_until_first_of_next_month,
                );
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
                date::NextDay::Week(next_day) => next_day.map(|(month, week)| {
                    let cache = self.days.cache_mut().unwrap();
                    *cache = Some(DayCache {
                        last_month_day: month,
                        last_weekday: week,
                        last_used: LastUsed::Week,
                    });
                    month
                }),
                date::NextDay::Both {
                    week: next_day_week,
                    month: next_day_month,
                } => {
                    let cache = self.days.cache_mut().unwrap();
                    let week_wins = |month, week| {
                        Some(DayCache {
                            last_month_day: month,
                            last_weekday: week,
                            last_used: LastUsed::Week,
                        })
                    };
                    let month_wins = |month| {
                        Some(DayCache {
                            last_month_day: month,
                            last_weekday: DaysInner::next_weekday_from_last(
                                new_days_week as u32,
                                (month - new_days_month) as u32,
                            ),
                            last_used: LastUsed::Month,
                        })
                    };
                    let both_win = |month, week| {
                        Some(DayCache {
                            last_month_day: month,
                            last_weekday: week,
                            last_used: LastUsed::Both,
                        })
                    };
                    let new_cache = match (next_day_month, next_day_week) {
                        (None, None) => None,
                        (None, Some((next_of_month, next_of_week))) => {
                            week_wins(next_of_month, next_of_week)
                        }
                        (Some(next), None) => month_wins(next),
                        (Some(next_from_month_ring), Some((next_from_week_ring, next_weekday))) => {
                            match next_from_month_ring.cmp(&next_from_week_ring) {
                                std::cmp::Ordering::Less => month_wins(next_from_month_ring),
                                std::cmp::Ordering::Equal => {
                                    both_win(next_from_month_ring, next_weekday)
                                }
                                std::cmp::Ordering::Greater => {
                                    week_wins(next_from_week_ring, next_weekday)
                                }
                            }
                        }
                    };
                    let next_day = new_cache.as_ref().map(|c| c.last_month_day);
                    *cache = new_cache;
                    next_day
                }
                date::NextDay::Month(next_day) => next_day,
            };
            if let Some(next) = next_day {
                result = Some(Cache {
                    last_day: next,
                    last_month: month,
                    last_year: self.year_unchecked(),
                });
                found = true;
            } else {
                first_run = false;
            }
        }

        result.and_then(|d| {
            NaiveDate::from_ymd_opt(d.last_year as i32, d.last_month as u32, d.last_day as u32)
        })
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
    pub fn next(&mut self, mut time_overflow: bool) -> Option<NaiveDate> {
        // First, grab the current day, month, and year
        // We can get the weekday from our day cache, if necessary.

        let days_month = self.days.last();
        let month = self.months.last();
        let year = self.year_unchecked();

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
            || days
                .query_both(CopyRing::is_empty, |(w, _)| w.is_empty())
                .any()
        {
            return Err(Error::EmptyRing);
        }

        if months.first().unwrap() < 1
            || months.last().unwrap() > 31
            || days
                .query_both(
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
                CronRing::arc_with_size(Arc::new([2, 5])),
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
            let a = d.first_after(true, 30, 6, 12, 2024);
        }
        let now = Utc::now();
        println!("{:?}", now - then);
    }
}
