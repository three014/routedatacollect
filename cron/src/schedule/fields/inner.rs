use crate::schedule::iterator::CopyRing;

#[derive(Clone, Debug)]
pub struct Seconds(CopyRing<u8>);

#[derive(Clone, Debug)]
pub struct Minutes(CopyRing<u8>);

#[derive(Clone, Debug)]
pub struct Hours(CopyRing<u8>);

#[derive(Clone, Debug)]
pub enum Days {
    Both {
        month: CopyRing<u8>,
        week: CopyRing<u8>,
    },
    Month(CopyRing<u8>),
    Week(CopyRing<u8>),
}

#[derive(Clone, Debug)]
pub struct Months(CopyRing<u8>);

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
        }
        if secs_overflow {
            overflow = self.0.checked_next().unwrap().1 || overflow
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
                        Ordering::Less => next_day_month,
                        Ordering::Equal => next_day_month, // Doesn't matter which one to return
                        Ordering::Greater => next_day_week,
                    },
                    (true, false) => {
                        // The month day was sooner, commit to the month day
                        next_day_month
                    }
                    (false, true) => {
                        // The weekday was sooner, commit to the weekday
                        next_day_week
                    }
                }
            }
            Days::Month(month) => {
                Self::next_day_of_the_month(
                    month,
                    hours_overflow,
                    day_of_month,
                    days_in_curr_month,
                )
            }
            Days::Week(week) => {
                Self::next_day_of_the_week(
                    week,
                    hours_overflow,
                    day_of_week,
                    day_of_month,
                    days_in_curr_month,
                )
            }
        }
    }

    pub fn next(&mut self, hours_overflow: bool) -> (u8, bool) {
        todo!()
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
}
