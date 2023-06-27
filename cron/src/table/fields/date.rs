use std::ops::{Deref, DerefMut};

use crate::{table::CronRing, CopyRing};

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

#[derive(Clone, Debug)]
pub struct Response<M, W>((Option<M>, Option<W>));

impl<M, W> Deref for Response<M, W> {
    type Target = (Option<M>, Option<W>);

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<M, W> DerefMut for Response<M, W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<M, W> Response<M, W> {
    pub fn map<F, G, B>(self, month_map: F, week_map: G) -> impl ExactSizeIterator<Item = B>
    where
        F: FnOnce(M) -> B,
        G: FnOnce(W) -> B,
    {
        struct Combined<T>((Option<T>, Option<T>));
        impl<T> Iterator for Combined<T> {
            type Item = T;

            fn next(&mut self) -> Option<Self::Item> {
                self.0 .0.take().or_else(|| self.0 .1.take())
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                match self.0 {
                    (None, None) => (0, Some(0)),
                    (Some(_), None) | (None, Some(_)) => (1, Some(1)),
                    (Some(_), Some(_)) => (2, Some(2)),
                }
            }
        }
        impl<T> ExactSizeIterator for Combined<T> {}

        Combined((self.0 .0.map(month_map), self.0 .1.map(week_map)))
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
        let _ = self.apply_both(CopyRing::reset, |(w, _)| w.reset());
    }

    pub(self) const fn num_weekdays_since(start_weekday: i16, end_weekday: i16) -> u8 {
        const DAYS_IN_A_WEEK: i16 = 7;
        ((end_weekday + DAYS_IN_A_WEEK - start_weekday) % DAYS_IN_A_WEEK) as u8
    }

    pub fn cache_mut(&mut self) -> Option<&mut Option<DayCache>> {
        self.apply_week(|(_, cache)| cache)
    }

    fn cache(&self) -> Option<&DayCache> {
        self.query_week(|(_, cache)| cache)
            .and_then(|cache| cache.as_ref())
    }

    /// Returns the last day saved in the cache, or the last day in the month
    /// copyring if no cache exists.
    pub fn last(&self) -> u8 {
        if let Some(cache) = self.cache() {
            cache.last_month_day
        } else {
            self.query_month(|month| month.peek_prev().unwrap())
                .unwrap()
        }
    }

    pub fn query_week<'a: 'b, 'b, W, U>(&'a self, week_fn: W) -> Option<U>
    where
        W: FnOnce(&'b (CronRing, Option<DayCache>)) -> U,
    {
        match self {
            DaysInner::Both { month: _, week } => Some(week_fn(week)),
            DaysInner::Week(week) => Some(week_fn(week)),
            _ => None,
        }
    }

    pub fn query_month<'a: 'b, 'b, M, U>(&'a self, month_fn: M) -> Option<U>
    where
        M: FnOnce(&'b CronRing) -> U,
    {
        match self {
            DaysInner::Both { month, week: _ } => Some(month_fn(month)),
            DaysInner::Month(month) => Some(month_fn(month)),
            _ => None,
        }
    }

    pub fn query_both<'a: 'b, 'b, M, W, T, U>(
        &'a self,
        month_fn: M,
        week_fn: W,
    ) -> Response<T, U>
    where
        M: FnOnce(&'b CronRing) -> T,
        W: FnOnce(&'b (CronRing, Option<DayCache>)) -> U,
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

    fn apply_both<'a: 'b, 'b, M, W, T, U>(
        &'a mut self,
        month_fn: M,
        week_fn: W,
    ) -> Response<T, U>
    where
        M: FnOnce(&'b mut CronRing) -> T,
        W: FnOnce(&'b mut (CronRing, Option<DayCache>)) -> U,
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
        days_in_month: u8,
    ) -> NextDay {
        self.reset();
        let result = self.apply_both(
            |month_ring| {
                month_ring
                    .binary_search_or_greater(&(days_month + time_overflow as u8))
                    .filter(|&(day, overflow)| {
                        !overflow && Self::check_for_end_of_month(day, days_in_month)
                    })
                    .map(|(day, _)| day)
            },
            |(week_ring, _)| {
                week_ring
                    .binary_search_or_greater(&(days_week + time_overflow as u8))
                    .map(|(day, _)| day)
                    .or_else(|| week_ring.next())
                    .map(|day| {
                        let day_of_month = days_month
                            + DaysInner::num_weekdays_since(days_week.into(), day.into());
                        let day_of_week = day;
                        (day_of_month, day_of_week)
                    })
                    .filter(|&(day, _)| Self::check_for_end_of_month(day, days_in_month))
            },
        );
        Self::handle_result_for_next_day(result)
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
        let result = self.apply_both(
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
                            + DaysInner::num_weekdays_since(last_weekday.into(), day.into());
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
        Self::handle_result_for_next_day(result)
    }

    fn handle_result_for_next_day(result: Response<Option<u8>, Option<(u8, u8)>>) -> NextDay {
        match result.0 {
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
