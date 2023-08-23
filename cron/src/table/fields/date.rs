use crate::{table::CronRing, CopyRing};
use std::ops::{Deref, DerefMut};

use super::Date;

#[derive(Clone, Debug)]
pub enum DaysInner {
    Both {
        month: CronRing,
        week: (CronRing, Option<DayCache>),
    },
    Month(CronRing),
    Week((CronRing, Option<DayCache>)),
}

#[derive(Clone, Debug)]
pub struct Days(DaysInner);

#[derive(Clone, Debug)]
pub struct Months(CronRing);

#[derive(Clone, Debug, Default)]
pub struct Years(Option<u32>);

/// A helper struct for the weekday's
/// part of the `Days` struct. Allows
/// the weekday ring to cache it's last
/// weekday and corresponding month-day
/// for future month day calculations.
///
/// The `month_day` `weekday`
/// fields are meant for the day of the
/// month that was calculated using the
/// weekday ring, not the month-day ring.
/// The month ring doesn't need to store
/// its own weekday, according to the
/// following logic:
///
/// If the last used day was
/// the month ring, then the stored
/// days are the first days that
/// the weekday can use next time. Every
/// time that the month ring gets selected
/// to be used as the next day, the
/// weekday ring needs to be updated with the
/// next available day so that subsequent
/// queries can be calculated.
#[derive(Clone, Debug, Default)]
pub struct DayCache {
    pub month_day: u8,
    pub weekday: u8,
    pub month: u8,
    pub year: u32,
    pub last: LastUsed,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum LastUsed {
    #[default]
    Week,
    Month,
    Both,
}

#[derive(Clone, Debug)]
pub struct Response<Month, Week>((Option<Month>, Option<Week>));

#[derive(Clone, Debug)]
pub struct TryMerge<B, T>(Result<B, Response<T, T>>);

impl<Month, Week> Response<Month, Week> {
    pub fn new(month: Option<Month>, week: Option<Week>) -> Self {
        Self((month, week))
    }

    pub fn map<M, W, T, U>(self, month_fn: M, week_fn: W) -> Response<T, U>
    where
        M: FnOnce(Month) -> T,
        W: FnOnce(Week) -> U,
    {
        let (m, w) = self.0;
        Response((m.map(month_fn), w.map(week_fn)))
    }

    pub fn map_week<W, U>(self, week_fn: W) -> Response<Month, U>
    where
        W: FnOnce(Week) -> U,
    {
        let (m, w) = self.0;
        Response((m, w.map(week_fn)))
    }

    pub fn inspect_week<W>(self, week_fn: W) -> Response<Month, Week>
    where
        W: FnOnce(&Week),
    {
        let (month, week) = self.0;
        if let Some(ref week) = week {
            week_fn(week);
        }
        Response((month, week))
    }
}

impl<T> Response<T, T> {
    /// Attempts to merge both the `Month` and `Week` generic types
    /// into one item of type `B`. If either the month or week
    /// doesn't exist, then the original response gets returned
    /// without the function applied to the data.
    pub fn try_merge<B, F>(self, f: F) -> TryMerge<B, T>
    where
        F: FnOnce(T, T) -> B,
    {
        match self {
            Response((Some(m), Some(w))) => TryMerge(Ok(f(m, w))),
            Response(data) => TryMerge(Err(Response(data))),
        }
    }
}

impl Response<bool, bool> {
    pub fn any(self) -> bool {
        match self.0 {
            (None, None) => true,
            (None, Some(w)) => w,
            (Some(m), None) => m,
            (Some(m), Some(w)) => m || w,
        }
    }
}

impl<B, T> TryMerge<B, T> {
    pub fn or_else_map<M, W>(self, month_fn: M, week_fn: W) -> Option<B>
    where
        M: FnOnce(T) -> B,
        W: FnOnce(T) -> B,
    {
        match self.0 {
            Ok(item) => Some(item),
            Err(response) => match response.0 {
                (None, None) => None,
                (None, Some(w)) => Some(week_fn(w)),
                (Some(m), None) => Some(month_fn(m)),
                _ => unreachable!("Should not have been created with both fields"),
            },
        }
    }
}

impl<T> TryMerge<T, T> {
    /// Returns the result of applying `try_merge` to the value
    /// if both fields existed, or just returns the inner value
    /// in the case that only one of the fields existed. This is
    /// a terminal operation.
    ///
    /// This function is available if and only iff `try_merge`
    /// returned the same type as the inner fields of `Response`.
    /// Otherwise, use `or_else_map` to convert the fields to
    /// the correct type.
    pub fn or(self) -> Option<T> {
        match self.0 {
            Ok(item) => Some(item),
            Err(response) => match response.0 {
                (None, None) => None,
                (None, Some(w)) => Some(w),
                (Some(m), None) => Some(m),
                _ => unreachable!("Should not have been created with none or both fields"),
            },
        }
    }
}

impl Days {
    pub fn week(ring: CronRing) -> Self {
        Self(DaysInner::Week((ring, None)))
    }

    pub fn month(ring: CronRing) -> Self {
        Self(DaysInner::Month(ring))
    }

    pub fn both(month: CronRing, week: CronRing) -> Self {
        Self(DaysInner::Both {
            month,
            week: (week, None),
        })
    }
}

impl Deref for Days {
    type Target = DaysInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Days {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DaysInner {
    pub fn reset(&mut self) {
        let _ = self.apply(CopyRing::reset, |(w, _)| w.reset());
    }

    pub(self) const fn num_weekdays_since(start_weekday: i16, end_weekday: i16) -> u8 {
        const DAYS_IN_A_WEEK: i16 = 7;
        ((end_weekday + DAYS_IN_A_WEEK - start_weekday) % DAYS_IN_A_WEEK) as u8
    }

    pub fn cache_mut(&mut self) -> Option<&mut DayCache> {
        self.apply_week(|(_, cache)| cache)
            .and_then(|cache| cache.as_mut())
    }

    pub fn set_cache(&mut self, new_cache: DayCache) {
        self.apply_week(|(_, cache)| *cache = Some(new_cache));
    }

    fn cache(&self) -> Option<&DayCache> {
        self.query_week(|(_, cache)| cache)
            .and_then(|cache| cache.as_ref())
    }

    /// Returns the last day saved in the cache, or the last day in the month
    /// copyring if no cache exists.
    pub fn last(&self) -> u8 {
        if let Some(cache) = self.cache() {
            cache.month_day
        } else {
            self.query_month(|month| month.peek_prev().unwrap())
                .unwrap()
        }
    }

    pub fn query_week<'a: 'b, 'b, W, U>(&'a self, week_fn: W) -> Option<U>
    where
        W: Fn(&'b (CronRing, Option<DayCache>)) -> U,
    {
        match self {
            DaysInner::Both { month: _, week } => Some(week_fn(week)),
            DaysInner::Week(week) => Some(week_fn(week)),
            _ => None,
        }
    }

    pub fn query_month<'a: 'b, 'b, M, U>(&'a self, month_fn: M) -> Option<U>
    where
        M: Fn(&'b CronRing) -> U,
    {
        match self {
            DaysInner::Both { month, week: _ } => Some(month_fn(month)),
            DaysInner::Month(month) => Some(month_fn(month)),
            _ => None,
        }
    }

    pub fn query<'a: 'b, 'b, M, W, T, U>(&'a self, month_fn: M, week_fn: W) -> Response<T, U>
    where
        M: Fn(&'b CronRing) -> T,
        W: Fn(&'b (CronRing, Option<DayCache>)) -> U,
    {
        match self {
            DaysInner::Both { month, week } => {
                Response((Some(month_fn(month)), Some(week_fn(week))))
            }
            DaysInner::Month(month) => Response((Some(month_fn(month)), None)),
            DaysInner::Week(week) => Response((None, Some(week_fn(week)))),
        }
    }

    fn apply_week<'a: 'b, 'b, W, U>(&'a mut self, week_fn: W) -> Option<U>
    where
        W: FnOnce(&'b mut (CronRing, Option<DayCache>)) -> U,
    {
        match self {
            DaysInner::Both { month: _, week } => Some(week_fn(week)),
            DaysInner::Week(week) => Some(week_fn(week)),
            _ => None,
        }
    }

    fn apply<'a: 'b, 'b, M, W, T, U>(&'a mut self, month_fn: M, week_fn: W) -> Response<T, U>
    where
        M: Fn(&'b mut CronRing) -> T,
        W: Fn(&'b mut (CronRing, Option<DayCache>)) -> U,
    {
        match self {
            DaysInner::Both { month, week } => {
                Response((Some(month_fn(month)), Some(week_fn(week))))
            }
            DaysInner::Month(month) => Response((Some(month_fn(month)), None)),
            DaysInner::Week(week) => Response((None, Some(week_fn(week)))),
        }
    }

    pub fn first_after(
        &mut self,
        time_overflow: bool,
        days_month: u8,
        days_week: u8,
        month: u8,
        year: u32,
    ) -> Option<u8> {
        self.reset();
        let days_in_month = Date::days_in_month(month, year);
        let first_after_for_month = |month_ring: &mut CronRing| {
            month_ring
                .binary_search_or_greater(&(days_month + time_overflow as u8))
                .filter(|&(day, overflow)| {
                    !overflow && Self::is_within_end_of_month(day, days_in_month)
                })
                .map(|(day, _)| day)
        };
        let first_after_for_week = |(week_ring, _): &mut (CronRing, _)| {
            week_ring
                .binary_search_or_greater(&(days_week + time_overflow as u8))
                .map(|(day, _)| {
                    let day_of_month =
                        days_month + DaysInner::num_weekdays_since(days_week.into(), day.into());
                    let day_of_week = day;
                    (day_of_month, day_of_week)
                })
        };
        self.apply(first_after_for_month, first_after_for_week)
            .inspect_week(|&weekday| {
                self.set_cache(Self::compute_cache(weekday.unwrap(), month, year))
            })
            .map_week(|weekday| {
                weekday
                    .map(|(day, _)| day)
                    .filter(|&day| Self::is_within_end_of_month(day, days_in_month))
            })
            .map(
                |month_day| month_day.map(|day| (day, LastUsed::Month)),
                |weekday| weekday.map(|day| (day, LastUsed::Week)),
            )
            .try_merge(Self::decide_field)
            .or()
            .expect("at least one field (month or week) to exist")
            .map(|(day, last_used)| {
                if let Some(cache) = self.cache_mut() {
                    cache.last = last_used;
                }
                day
            })
    }

    fn decide_field(
        month_opt: Option<(u8, LastUsed)>,
        week_opt: Option<(u8, LastUsed)>,
    ) -> Option<(u8, LastUsed)> {
        Response::new(month_opt, week_opt)
            .try_merge(|month_opt, week_opt| {
                let (month_day, _) = month_opt;
                let (weekday, _) = week_opt;
                match month_day.cmp(&weekday) {
                    std::cmp::Ordering::Less => month_opt,
                    std::cmp::Ordering::Equal => (month_day, LastUsed::Both),
                    std::cmp::Ordering::Greater => week_opt,
                }
            })
            .or()
    }

    fn compute_cache(day: (u8, u8), month: u8, year: u32) -> DayCache {
        let days_in_month = Date::days_in_month(month, year);
        let (month_day, weekday) = day;
        if month_day > days_in_month {
            // Overflow
            let new_day = month - days_in_month;
            let (new_month, month_overflow) = {
                let new_month = month + 1;
                if new_month > 12 {
                    (1, true)
                } else {
                    (new_month, false)
                }
            };
            let new_year = year + month_overflow as u32;
            DayCache {
                month_day: new_day,
                weekday,
                month: new_month,
                year: new_year,
                last: LastUsed::Week,
            }
        } else {
            DayCache {
                month_day,
                weekday,
                month,
                year,
                last: LastUsed::Week,
            }
        }
    }

    /// Returns the next days in the struct with an indication as to
    /// which internal buffer provided which day. Unlike `first_after`,
    /// however, this function assumes there was overflow, and will attempt
    /// to rotate the internal buffers to their next items.
    pub fn next(&mut self, days_in_month: u8) -> Response<Option<u8>, Option<(u8, u8)>> {
        let (month_overflow, week_overflow) = if let Some(cache) = self.cache() {
            match cache.last {
                LastUsed::Week => (false, true),
                LastUsed::Month => (true, false),
                LastUsed::Both => (true, true),
            }
        } else {
            (true, false) // No cache means only days of the month
        };
        let next_from_month = |month_ring| {
            Some(super::next(month_ring, month_overflow))
                .filter(|&(day, overflow)| {
                    !overflow && Self::is_within_end_of_month(day, days_in_month)
                })
                .map(|(day, _)| day)
        };
        let next_from_week = |(week_ring, cache): &mut (_, Option<DayCache>)| {
            let cache = cache.as_ref().unwrap();
            Some(super::next(week_ring, week_overflow))
                .map(|(day, _)| {
                    let last_month_day = cache.month_day;
                    let last_weekday = cache.weekday;
                    let day_of_month = last_month_day
                        + DaysInner::num_weekdays_since(last_weekday.into(), day.into());
                    let day_of_week = day;
                    (day_of_month, day_of_week)
                })
                .filter(|&(day, _)| Self::is_within_end_of_month(day, days_in_month))
        };
        self.apply(next_from_month, next_from_week)
    }

    fn is_within_end_of_month(day: u8, days_in_month: u8) -> bool {
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
        super::after(&mut self.0, false, month)
    }

    pub fn next(&mut self) -> (u8, bool) {
        super::next(&mut self.0, true)
    }

    pub fn last(&self) -> u8 {
        self.0.peek_prev().unwrap()
    }

    pub fn reset(&mut self) {
        self.0.reset()
    }
}

#[cfg(test)]
mod test {
    use crate::table::fields::date::DaysInner;

    #[test]
    fn next_weekday_from_last_works() {
        let start = 0;
        let n = 7;
        assert_eq!(0, DaysInner::next_weekday_from_last(start, n));

        let start = 1;
        let n = 13;
        assert_eq!(0, DaysInner::next_weekday_from_last(start, n));

        let start = 5;
        let n = 0;
        assert_eq!(5, DaysInner::next_weekday_from_last(start, n));
    }

    #[test]
    fn days_between_weekdays_works() {
        let l = 0;
        let r = 1;
        assert_eq!(1, DaysInner::num_weekdays_since(l, r));

        let l = 5;
        let r = 1;
        assert_eq!(3, DaysInner::num_weekdays_since(l, r));

        let l = 2;
        let r = 6;
        assert_eq!(4, DaysInner::num_weekdays_since(l, r));

        let l = 6;
        let r = 5;
        assert_eq!(6, DaysInner::num_weekdays_since(l, r));
    }
}
