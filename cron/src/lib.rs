pub enum Error {
    Empty,
    WrongNumberOfFields,
    InvalidMacro,
    Unknown,
}

pub mod schedule {
    use self::{
        fields::FieldTable,
        iterator::{OwnedScheduleIter, ScheduleIter},
    };
    use crate::Error;
    use chrono::{DateTime, Datelike, TimeZone, Utc};
    use std::str::FromStr;

    mod fields;
    mod iterator;

    #[derive(Clone, Debug)]
    pub struct Schedule {
        fields: Box<FieldTable>,
    }

    impl Schedule {
        pub fn iter_with_timezone<Tz: TimeZone + Clone + 'static>(
            &mut self,
            tz: Tz,
        ) -> impl Iterator<Item = DateTime<Tz>> + '_ {
            let first = self.recalibrate(&tz);
            ScheduleIter::new(self, first)
        }

        fn recalibrate<Tz: TimeZone + Clone + 'static>(&mut self, tz: &Tz) -> Option<DateTime<Tz>> {
            let first = self.fields.after(&Utc::now().with_timezone(tz));
            first.and_then(|dt| dt.and_local_timezone(tz.clone()).earliest())
        }

        pub fn into_iter_with_timezone<Tz: TimeZone + 'static>(
            mut self,
            tz: Tz,
        ) -> impl Iterator<Item = DateTime<Tz>> {
            let first = self.recalibrate(&tz);
            OwnedScheduleIter::new(self, first)
        }

        fn next<Tz: TimeZone + 'static>(
            &mut self,
            datetime: &DateTime<Tz>,
        ) -> Option<DateTime<Tz>> {
            let month = datetime.month() as u8;
            let year = datetime.year() as u32;
            self.fields
                .next(month, year)
                .and_then(|dt| dt.and_local_timezone(datetime.timezone()).earliest())
        }
    }

    impl FromStr for Schedule {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let fields: Vec<&str> = s.split_whitespace().collect();
            match fields.len() {
                0 => Err(Error::Empty),
                1 => {
                    let mut maybe_macro = fields[0].chars();
                    maybe_macro
                        .next()
                        .ok_or(Error::Unknown)?
                        .eq(&'@')
                        .then(|| match maybe_macro.next().ok_or(Error::InvalidMacro)? {
                            'y' | 'a' => Ok(Schedule {
                                fields: Box::new(annually())
                            }),
                            'm' => Ok(Schedule {
                                fields: Box::new(monthly())
                            }),
                            'w' => Ok(Schedule {
                                fields: Box::new(weekly())
                            }),
                            'd' => Ok(Schedule {
                                fields: Box::new(daily())
                            }),
                            'h' => Ok(Schedule {
                                fields: Box::new(hourly())
                            }),
                            _ => Err(Error::InvalidMacro),
                        })
                        .unwrap_or(Err(Error::WrongNumberOfFields))
                }
                5 => unimplemented!("Will eventually be the equivalent of the 6-field version, but with '00' for seconds."),
                6 => todo!(),
                _ => Err(Error::WrongNumberOfFields),
            }
        }
    }

    fn hourly() -> FieldTable {
        FieldTable::builder()
            .with_secs(0)
            .with_mins(0)
            .with_hrs_iter(0..24)
            .with_days_of_the_month_only_iter(1..=31)
            .with_months_iter(1..=12)
            .build()
            .unwrap()
    }

    fn daily() -> FieldTable {
        FieldTable::builder()
            .with_secs(0)
            .with_mins(0)
            .with_hrs(0)
            .with_days_of_the_month_only_iter(1..=31)
            .with_months_iter(1..=12)
            .build()
            .unwrap()
    }

    fn weekly() -> FieldTable {
        FieldTable::builder()
            .with_secs(0)
            .with_mins(0)
            .with_hrs(0)
            .with_days_of_the_week_only(0)
            .with_months_iter(1..=12)
            .build()
            .unwrap()
    }

    fn annually() -> FieldTable {
        FieldTable::builder()
            .with_secs(0)
            .with_mins(0)
            .with_hrs(0)
            .with_days_of_the_month_only(1)
            .with_months(1)
            .build()
            .unwrap()
    }

    fn monthly() -> FieldTable {
        FieldTable::builder()
            .with_secs(0)
            .with_mins(0)
            .with_hrs(0)
            .with_days_of_the_month_only(1)
            .with_months_iter(1..=12)
            .build()
            .unwrap()
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
            year % 400 == 0
        } else {
            true
        }
    } else {
        false
    }
}
