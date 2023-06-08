use self::days::Spec;
use super::iterator::CopyRing;
use chrono::{DateTime, TimeZone};

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
/// Days of the Month and Week, in one struct.
struct Days {
    month: CopyRing<u8>,
    week: CopyRing<u8>,
    spec: Option<Spec>,
}

#[derive(Clone, Debug)]
struct Months(CopyRing<u8>);

mod days {
    #[derive(Clone, Copy, Debug)]
    pub enum Spec {
        MonthAndWeek,
        MonthOnly,
        WeekOnly,
    }
}

pub struct Init {
    pub secs: CopyRing<u8>,
    pub mins: CopyRing<u8>,
    pub hrs: CopyRing<u8>,
    pub days_of_the_month: CopyRing<u8>,
    pub months: CopyRing<u8>,
    pub days_of_the_week: CopyRing<u8>,
    pub days_spec: Option<Spec>,
}

impl FieldTable {
    pub fn new(init: Init) -> Self {
        Self {
            secs: Seconds(init.secs),
            mins: Minutes(init.mins),
            hours: Hours(init.hrs),
            days: Days {
                month: init.days_of_the_month,
                week: init.days_of_the_week,
                spec: init.days_spec,
            },
            months: Months(init.months),
            years_from_next: 0,
        }
    }

    pub fn after<Tz: TimeZone + 'static>(&mut self, date_time: &DateTime<Tz>) -> DateTime<Tz> {
        todo!()
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
    pub fn first_after(&mut self, secs: u32) -> (u8, bool) {
        let mut found = false;
        self.0.reset();
        for seconds in self.0.until_start() {
            if seconds as u32 >= secs {
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
    pub fn first_after(&mut self, mins: u32, secs_overflow: bool) -> (u8, bool) {
        let mut found = false;
        self.0.reset();
        if secs_overflow {
            self.0.rotate_left(1)
        };
        for minutes in self.0.one_cycle() {
            if minutes as u32 >= mins {
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
    pub fn first_after(&mut self, hrs: u32, mins_overflow: bool) -> (u8, bool) {
        let mut found = false;
        self.0.reset();
        if mins_overflow {
            self.0.rotate_left(1)
        };
        for hours in self.0.one_cycle() {
            if hours as u32 >= hrs {
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
        // What to do:

        // Synchronize the day of the month with the day of the week.
        // Whatever day it is will be the starting point for the sync.

        // IDEA: First, find the month day. If we find it before we finish
        // wrapping around, then that's our day. To sync with the weekdays,
        // subtract this day from the given month day, then advance the
        // weekdays by at least this amount. That way, calling `next` or `peek`
        // on the weekday buffer will give the closest weekday after the current
        // date.

        // HOWEVER: That doesn't account for the possibility of the weekday
        // being sooner than the month day. So...

        let days_in_curr_month = crate::days_in_a_month(month, year);
        let next = match self.spec {
            Some(Spec::MonthAndWeek) => {
                let next_day_week =
                    self.next_day_of_the_week(hours_overflow, day_of_week, days_in_curr_month);
                let next_day_month =
                    self.next_day_of_the_month(hours_overflow, day_of_month, days_in_curr_month);

                match (next_day_week.1, next_day_month.1) {
                    (true, true) | (false, false) => {
                        if next_day_month.0 < next_day_week.0 {
                            self.month.rotate_left(1);
                            next_day_month
                        } else if next_day_month.0 > next_day_week.0 {
                            self.week.rotate_left(1);
                            next_day_week
                        } else {
                            self.month.rotate_left(1);
                            self.week.rotate_left(1);
                            next_day_month
                        }
                    }
                    (true, false) => {
                        // The month day was sooner, commit to the month day
                        self.month.rotate_left(1);
                        (next_day_month.0, false)
                    }
                    (false, true) => {
                        // The weekday was sooner, commit to the weekday
                        self.week.rotate_left(1);
                        (next_day_week.0, false)
                    }
                }
            }
            Some(Spec::WeekOnly) => {
                let next =
                    self.next_day_of_the_week(hours_overflow, day_of_week, days_in_curr_month);
                self.week.rotate_left(1);
                next
            }
            None | Some(Spec::MonthOnly) => {
                let next =
                    self.next_day_of_the_month(hours_overflow, day_of_month, days_in_curr_month);
                self.month.rotate_left(1);
                next
            }
        };

        todo!()
    }

    fn next_day_of_the_month(
        &mut self,
        hours_overflow: bool,
        day_of_month: u8,
        days_in_curr_month: u8,
    ) -> (u8, bool) {
        let mut found = false;
        self.month.reset();
        if hours_overflow {
            self.month.rotate_right(1)
        }
        for day in self.month.one_cycle() {
            if day >= day_of_month {
                found = true;
                break;
            }
        }
        let next_day_month = if found {
            self.month.rotate_right(1);
            let next = self.month.peek().unwrap();
            if next > days_in_curr_month {
                self.month.reset();
                (self.month.peek().unwrap(), true)
            } else {
                (next, false)
            }
        } else {
            (self.month.peek().unwrap(), true)
        };
        next_day_month
    }

    fn next_day_of_the_week(
        &mut self,
        hours_overflow: bool,
        day_of_week: u8,
        days_in_curr_month: u8,
    ) -> (u8, bool) {
        let mut w_found = false;
        self.week.reset();
        if hours_overflow {
            self.week.rotate_right(1)
        }
        for weekday in self.week.one_cycle() {
            if weekday >= day_of_week {
                w_found = true;
                break;
            }
        }
        if w_found {
            self.week.rotate_right(1)
        };
        let weekday = self.week.peek().unwrap();
        let days_since_now = Self::num_weekdays_since(day_of_week as i8, weekday as i8);
        let next_day_week = {
            let next_day_unmodded = day_of_week + days_since_now;
            if next_day_unmodded > days_in_curr_month {
                (next_day_unmodded % days_in_curr_month, true)
            } else {
                (next_day_unmodded, false)
            }
        };
        next_day_week
    }

    fn num_weekdays_since(first_weekday: i8, second_weekday: i8) -> u8 {
        /*
        Given the length, the first index, and the
        number of steps to take, where n >= 0,
        we can find the second index like so:

            I2 = (I1 + n) % L

        Quotient Remainder theorem:

            if A == B * Q + R where 0 <= R < B,
            then A % B == R

                A    % B == R
            (I1 + n) % L == I2

            0 <= I2 < L is what we need to assert.

        Well,
            0 < L will always be true for us, since a field
            needs at least 1 value.

            I2 < L will always be true for us, because an
            index can't be == to the length (due to the nature
            of 0-index arrays).

            0 <= I2 will always be true, since the index
            of an array can be anywhere from 0 to L - 1.

        So we're good there! Let's continue:

            (I1 + n) % L == I2 =>
            I1 + n = L * Q + I2
            n = (L * Q + I2) - I1, Q in the set of integers.

        What would Q be?
        Using the definition of Modulo, we have that

            given A and B, with A > 0,
            A mod B == R <-> A == B * Q + R

        furthermore,

            A div B == Q <-> A == B * Q + R

        where 'div' is the / operator! And we already know
        that A == B * Q + R is true from earlier, so
        we can find Q using:

            (I1 + n) / L = Q

        Ah. And we're back to the first problem. :D
        But we're not done yet. Maybe instead, we could specify
        a range of values to check? In that case, we'd
        at least have a bound to check for.

            Let's put a bound on n: 0 <= n < L

        After some testing, we can see that because our
        indices are bounded by 0 and L, we can actually
        narrow Q down to being either 0 or 1. This means
        we only have to test which final answer n is within
        the range of [0, L).

        But knowing this, we can take one more step. By assuming
        Q to be 1, and eliminating the multiplication altogether,
        we can do this:

            (L + I2 - I1) % L

        which satisfies our needs and only needs one calculation,
        no branching.

        But ACTUALLY, if we can assume Q == 1, then we can assume
        Q to be any integer, since we're using the % operator anyway.

        Which means we can assume Q to be 0, and take away the first
        L, reducing our calculation to

            (I2 - I1) % L

        And That's Our Answer.
         */
        let days_in_a_week = 7i8;
        let y = days_in_a_week + second_weekday - first_weekday;
        y as u8
    }
}


#[cfg(test)]
mod test {

}