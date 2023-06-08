pub enum Error {
    Empty,
    WrongNumberOfFields,
    InvalidMacro,
    Unknown,
}

mod schedule {
    use self::iterator::{OwnedScheduleIter, ScheduleIter};
    use crate::{schedule::iterator::CopyRing, Error};
    use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
    use std::str::FromStr;

    mod iterator;
    mod fields;

    type Field = CopyRing<u32>;

    #[derive(Clone, Debug)]
    pub struct Schedule {
        fields: Vec<Field>,
        years_after_curr: u8,
        days_in_curr_month: u8,
    }


    impl Schedule {
        pub fn iter_with_timezone<Tz: TimeZone + 'static>(
            &mut self,
            tz: Tz,
        ) -> impl Iterator<Item = DateTime<Tz>> + '_ {
            self.recalibrate(&tz);
            ScheduleIter::new(self, tz)
        }

        fn next_sec(&mut self) {
            todo!()
        }

        fn next_min(&mut self) {
            todo!()
        }

        fn next_hour(&mut self) {
            todo!()
        }

        fn next_day_of_the_month(&mut self) {
            todo!()
        }

        fn next_month<Tz: TimeZone + 'static>(&mut self, now: &DateTime<Tz>) {
            let month = now.month();
            let f_month = &mut self.fields[4];
            let mut found = false;
            for m in f_month.one_cycle() {
                if month >= m {
                    found = true;
                    break;
                }
            }
            if found {
                f_month.rotate_right(2);
            } else {
                f_month.reset();
                self.years_after_curr += 1;
            }
        }

        fn next_day_of_the_week(&mut self) {
            todo!()
        }

        fn recalibrate<Tz: TimeZone + 'static>(&mut self, tz: &Tz) {}

        // fn recalibrate<Tz: TimeZone + 'static>(&mut self, tz: &Tz) {
        //     let now = Utc::now().with_timezone(tz);
        //     let second = now.second();
        //     let minute = now.minute();
        //     let hour = now.hour();
        //     let day_of_the_month = now.day();
        //     let month = now.month();
        //     let day_of_the_week = now.weekday();

        //     // Reset all fields to start
        //     self.fields[SECOND].reset();
        //     self.fields[MINUTE].reset();
        //     self.fields[HOUR].reset();
        //     self.fields[DAY_OF_THE_MONTH].reset();
        //     self.fields[MONTH].reset();
        //     self.fields[DAY_OF_THE_WEEK].reset();

        //     let mut s_second = false;
        //     let mut s_minute = false;
        //     let mut s_hour = false;
        //     let mut s_day_of_the_month = false;
        //     let mut s_month = false;
        //     let mut s_day_of_the_week = false;

        //     // For each field, check if there's a value that's
        //     // higher than the corresponding `now` time. If there
        //     // is, then use that value for the datetime and advance
        //     // the field to the next one.
        //     //
        //     // If the field cycles through and wasn't higher than
        //     // `now`, use the first field, and advance the next
        //     // field. If that causes the next field to wrap back around,
        //     // advance the next field, and so on.
        //     //
        //     // Make sure to account for the varying number of days
        //     // in a month, and that includes leap years as well.
        //     for sec in self.fields[SECOND].one_cycle() {
        //         if sec >= second {
        //             s_second = true;
        //             break;
        //         }
        //     }
        //     if !s_second {
        //         self.fields[MINUTE].next();
        //         self.fields[SECOND].reset();
        //     } else {
        //         self.fields[SECOND].prev();
        //         self.fields[SECOND].prev();
        //     }

        //     for min in self.fields[MINUTE].one_cycle() {
        //         if min >= minute {
        //             s_minute = true;
        //             break;
        //         }
        //     }
        //     if !s_minute {
        //         self.fields[HOUR].next();
        //         self.fields[MINUTE].reset();
        //     } else {
        //         self.fields[MINUTE].prev();
        //         self.fields[MINUTE].prev();
        //     }

        //     for hr in self.fields[HOUR].one_cycle() {
        //         if hr >= hour {
        //             s_hour = true;
        //             break;
        //         }
        //     }
        //     if !s_hour {
        //         self.fields[DAY_OF_THE_MONTH].next();
        //         self.fields[DAY_OF_THE_WEEK].next();
        //         self.fields[HOUR].reset();
        //     } else {
        //         self.fields[HOUR].prev();
        //         self.fields[HOUR].prev();
        //     }

        //     for day_of_month in self.fields[DAY_OF_THE_MONTH].one_cycle() {
        //         if day_of_month >= day_of_the_month {
        //             s_day_of_the_month = true;
        //             break;
        //         }
        //     }
        //     if !s_day_of_the_month {
        //         self.fields[MONTH].next();
        //         self.fields[DAY_OF_THE_MONTH].reset();
        //     } else {
        //         self.fields[DAY_OF_THE_MONTH].prev();
        //         self.fields[DAY_OF_THE_MONTH].prev();
        //     }

        //     for mth in self.fields[MONTH].one_cycle() {
        //         if mth >= month {
        //             s_month = true;
        //             break;
        //         }
        //     }
        //     if !s_month {
        //         self.years_after_curr += 1;
        //         self.fields[MONTH].reset();
        //     } else {
        //         self.fields[MONTH].prev();
        //         self.fields[MONTH].prev();
        //     }

        //     for day_of_week in self.fields[DAY_OF_THE_WEEK].one_cycle() {
        //         if day_of_week >= day_of_the_week.num_days_from_sunday() {
        //             s_day_of_the_week = true;
        //             break;
        //         }
        //     }
        //     if !s_day_of_the_week {
        //         self.fields[DAY_OF_THE_WEEK].reset();
        //     }
        // }

        pub fn into_iter_with_timezone<Tz: TimeZone + 'static>(
            mut self,
            tz: Tz,
        ) -> impl Iterator<Item = DateTime<Tz>> {
            self.recalibrate(&tz);
            OwnedScheduleIter::new(self, tz)
        }

        fn next<Tz: TimeZone + 'static>(&mut self, tz: &Tz) -> DateTime<Tz> {
            todo!()
        }
    }

    impl FromStr for Schedule {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let fields: Vec<&str> = s.split_whitespace().collect();
            Ok(match fields.len() {
            0 => Err(Error::Empty),
            1 => {
                let mut maybe_macro = fields[0].chars();
                maybe_macro
                    .next()
                    .ok_or(Error::Unknown)?
                    .eq(&'@')
                    .then(|| match maybe_macro.next().ok_or(Error::InvalidMacro)? {
                        'y' | 'a' => Ok(Schedule {
                            fields: vec![
                                Field::from(0),
                                Field::from(0),
                                Field::from(0),
                                Field::from(1),
                                Field::from(1),
                                Field::from_iter(0..=6)
                            ],
                            years_after_curr: 1,
                            days_in_curr_month: 31
                        }),
                        'm' => Ok(Schedule {
                            fields: vec![
                                Field::from(0),
                                Field::from(0),
                                Field::from(0),
                                Field::from(1),
                                Field::from_iter(1..=12),
                                Field::from_iter(0..=6)
                            ],
                            years_after_curr: 0,
                            days_in_curr_month: 31
                        }),
                        'w' => Ok(Schedule {
                            fields: vec![
                                Field::from(0),
                                Field::from(0),
                                Field::from(0),
                                Field::from_iter(1..=31),
                                Field::from_iter(1..=12),
                                Field::from(0),
                            ],
                            years_after_curr: 0,
                            days_in_curr_month: 31
                        }),
                        'd' => Ok(Schedule {
                            fields: vec![
                                Field::from(0),
                                Field::from(0),
                                Field::from(0),
                                Field::from_iter(1..=31),
                                Field::from_iter(1..=12),
                                Field::from_iter(0..=6)
                            ],
                            years_after_curr: 0,
                            days_in_curr_month: 31
                        }),
                        'h' => Ok(Schedule {
                            fields: vec![
                                Field::from(0),
                                Field::from(0),
                                Field::from_iter(0..24),
                                Field::from_iter(1..=31),
                                Field::from_iter(1..=12),
                                Field::from_iter(0..7)
                            ],
                            years_after_curr: 0,
                            days_in_curr_month: 31
                        }),
                        _ => Err(Error::InvalidMacro),
                    })
                    .unwrap_or(Err(Error::WrongNumberOfFields))
            }
            5 => unimplemented!("Will eventually be the equivalent of the 6-field version, but with '00' for seconds."),
            6 => todo!(),
            _ => Err(Error::WrongNumberOfFields),
        }?)
        }
    }

    #[cfg(test)]
    mod tests {}
}

fn days_in_a_month(month: u8, year: u32) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => {
            panic!("Number has to be from 1 - 12, corresponding to the months of the year.")
        }
    }
}

fn is_leap_year(year: u32) -> bool {
    if year % 4 == 0 {
        if year % 100 == 0 {
            if year % 400 == 0 {
                true
            } else {
                false
            }
        } else {
            true
        }
    } else {
        false
    }
}
