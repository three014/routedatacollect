use super::iterator::CopyRing;
use chrono::{DateTime, TimeZone};

pub enum Error {
    MissingField,
    EmptyRing,
    OutOfRange,
    Unsorted,
}

#[derive(Clone, Debug)]
pub struct FieldTable {
    secs: Seconds,
    mins: Minutes,
    hours: Hours,
    days: Days,
    months: Months,
    years_from_next: u8,
}

#[derive(Clone, Debug)]
struct Seconds(CopyRing<u8>);

#[derive(Clone, Debug)]
struct Minutes(CopyRing<u8>);

#[derive(Clone, Debug)]
struct Hours(CopyRing<u8>);

#[derive(Clone, Debug)]
enum Days {
    Both {
        month: CopyRing<u8>,
        week: CopyRing<u8>,
    },
    Month(CopyRing<u8>),
    Week(CopyRing<u8>),
}

#[derive(Clone, Debug)]
struct Months(CopyRing<u8>);

pub struct Builder {
    secs: Option<CopyRing<u8>>,
    mins: Option<CopyRing<u8>>,
    hrs: Option<CopyRing<u8>>,
    days: Option<Days>,
    months: Option<CopyRing<u8>>,
}

impl Builder {
    pub fn with_secs(&mut self, secs: impl Into<CopyRing<u8>>) -> &mut Self {
        self.secs = Some(secs.into());
        self
    }

    pub fn with_mins(&mut self, mins: impl Into<CopyRing<u8>>) -> &mut Self {
        self.mins = Some(mins.into());
        self
    }

    pub fn with_hrs(&mut self, hrs: impl Into<CopyRing<u8>>) -> &mut Self {
        self.hrs = Some(hrs.into());
        self
    }

    pub fn with_days_of_the_month_only(&mut self, days: impl Into<CopyRing<u8>>) -> &mut Self {
        self.days = Some(Days::Month(days.into()));
        self
    }

    pub fn with_days_of_the_week_only(&mut self, days: impl Into<CopyRing<u8>>) -> &mut Self {
        self.days = Some(Days::Week(days.into()));
        self
    }

    pub fn with_days_of_both(
        &mut self,
        week: impl Into<CopyRing<u8>>,
        month: impl Into<CopyRing<u8>>,
    ) -> &mut Self {
        self.days = Some(Days::Both {
            month: month.into(),
            week: week.into(),
        });
        self
    }

    pub fn with_months(&mut self, months: Option<CopyRing<u8>>) -> &mut Self {
        self.months = months;
        self
    }

    pub fn build(self) -> Result<FieldTable, Error> {
        if self.secs.is_none()
            || self.mins.is_none()
            || self.hrs.is_none()
            || self.days.is_none()
            || self.months.is_none()
        {
            return Err(Error::MissingField);
        }

        todo!()
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            secs: Default::default(),
            mins: Default::default(),
            hrs: Default::default(),
            days: Default::default(),
            months: Default::default(),
        }
    }
}

impl FieldTable {
    pub fn after<Tz: TimeZone + 'static>(&mut self, date_time: &DateTime<Tz>) -> DateTime<Tz> {
        todo!()
    }

    pub fn builder() -> Builder {
        Builder::default()
    }
}

impl Seconds {
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
        }
        if secs_overflow {
            overflow = self.0.checked_next().unwrap().1 || overflow
        }
        if found {
            self.0.rotate_right(1);
            let final_overflow = secs_overflow && overflow;
            (self.0.next().unwrap(), final_overflow)
        } else {
            (self.0.next().unwrap(), true)
        }
    }

    /// Returns the next minute in the inner
    /// buffer, along with whether overflow
    /// occurred. For minutes, overflow
    /// occurs when the minutes passes 59
    /// and wraps back to 0.
    pub fn next(&mut self) -> (u8, bool) {
        self.0.checked_next().unwrap()
    }
}

impl Hours {
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
            (self.0.next().unwrap(), final_overflow)
        } else {
            (self.0.next().unwrap(), true)
        }
    }

    /// Returns the next hour in the inner
    /// buffer, along with whether overflow
    /// occurred. For hours, overflow
    /// occurs when the hours passes 23
    /// and wraps back to 0.
    pub fn next(&mut self) -> (u8, bool) {
        self.0.checked_next().unwrap()
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
        let days_in_curr_month = crate::days_in_a_month(month, year);
        match self {
            Days::Both { month, week } => {
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
                    (true, true) | (false, false) => match next_day_month.0.cmp(&next_day_week.0) {
                        std::cmp::Ordering::Less => {
                            month.rotate_left(1);
                            next_day_month
                        }
                        std::cmp::Ordering::Equal => {
                            month.rotate_left(1);
                            week.rotate_left(1);
                            next_day_month
                        }
                        std::cmp::Ordering::Greater => {
                            week.rotate_left(1);
                            next_day_week
                        }
                    },
                    (true, false) => {
                        // The month day was sooner, commit to the month day
                        month.rotate_left(1);
                        next_day_month
                    }
                    (false, true) => {
                        // The weekday was sooner, commit to the weekday
                        week.rotate_left(1);
                        next_day_week
                    }
                }
            }
            Days::Month(month) => {
                let next = Self::next_day_of_the_month(
                    month,
                    hours_overflow,
                    day_of_month,
                    days_in_curr_month,
                );
                month.rotate_left(1);
                next
            }
            Days::Week(week) => {
                let next = Self::next_day_of_the_week(
                    week,
                    hours_overflow,
                    day_of_week,
                    day_of_month,
                    days_in_curr_month,
                );
                week.rotate_left(1);
                next
            }
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

    fn num_weekdays_since(first_weekday: i8, second_weekday: i8) -> u8 {
        let days_in_a_week = 7i8;
        let diff = days_in_a_week + second_weekday - first_weekday;
        (diff % days_in_a_week) as u8
    }
}



impl Months {
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
            (self.0.next().unwrap(), final_overflow)
        } else {
            (self.0.next().unwrap(), true)
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Days, Minutes, Seconds};
    use crate::schedule::iterator::CopyRing;
    use chrono::{Datelike, Timelike, Utc};
    use rand::Rng;

    const THRESHOLD: i32 = 50;
    const UPPER: i32 = 100;

    fn gen_range_days_of_month() -> impl Iterator<Item = u8> {
        let mut v = vec![];
        let mut rng = rand::thread_rng();
        for i in 1u8..=31 {
            if rng.gen::<i32>() % UPPER > THRESHOLD {
                v.push(i);
            }
        }
        if v.is_empty() {
            v.push(rng.gen::<u8>() % 31)
        }
        v.into_iter()
    }

    fn gen_range_days_of_week() -> impl Iterator<Item = u8> {
        let mut v = vec![];
        let mut rng = rand::thread_rng();
        for i in 0u8..7 {
            if rng.gen::<i32>() % UPPER > THRESHOLD {
                v.push(i);
            }
        }
        if v.is_empty() {
            v.push(rng.gen::<u8>() % 7)
        }
        v.into_iter()
    }

    fn gen_range_hours_or_mins_or_secs() -> impl Iterator<Item = u8> {
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
        v.into_iter()
    }

    #[test]
    fn num_weekdays_since_returns_correct_day() {
        let sun_to_fri = Days::num_weekdays_since(0, 5);
        assert_eq!(5, sun_to_fri);

        let fri_to_sun = Days::num_weekdays_since(5, 0);
        assert_eq!(2, fri_to_sun);

        let wed_to_tues = Days::num_weekdays_since(3, 2);
        assert_eq!(6, wed_to_tues);

        let thurs_to_thurs = Days::num_weekdays_since(4, 4);
        assert_eq!(0, thurs_to_thurs);
    }

    #[test]
    fn first_after_works_for_secs() {
        let mut seconds = Seconds(CopyRing::from_iter(gen_range_hours_or_mins_or_secs()));
        let now = Utc::now();

        let next = seconds.first_after(now.second() as u8);
        match next.1 {
            true => assert!((next.0 as u32) < now.second()),
            false => assert!((next.0 as u32) >= now.second()),
        }
    }

    #[test]
    fn first_after_works_for_mins_no_overflow() {
        let mut minutes = Minutes(CopyRing::from_iter(gen_range_hours_or_mins_or_secs()));
        let now = Utc::now();

        let next = minutes.first_after(now.minute() as u8, false);
        match next.1 {
            true => assert!((next.0 as u32) < now.minute()),
            false => assert!((next.0 as u32) >= now.minute()),
        }
    }

    #[test]
    fn first_after_works_for_mins_overflow() {
        let mut minutes = Minutes(CopyRing::from_iter(gen_range_hours_or_mins_or_secs()));
        let now = Utc::now();

        let next = minutes.first_after(now.minute() as u8, true);
        dbg!(next);
        dbg!(minutes);
        match next.1 {
            true => assert!((next.0 as u32) < now.minute()),
            false => assert!((next.0 as u32) >= now.minute()),
        }
    }

    #[test]
    fn first_after_days_both_spec() {
        let mut days = Days::Both {
            week: CopyRing::from_iter(gen_range_days_of_week()),
            month: CopyRing::from_iter(gen_range_days_of_month()),
        };

        let now = Utc::now();
        let next = days.first_after(
            now.day() as u8,
            now.weekday().num_days_from_sunday() as u8,
            false,
            now.month() as u8,
            now.year() as u32,
        );

        //dbg!(days);
        //println!("{}, {:?}", now, next);
    }
}
