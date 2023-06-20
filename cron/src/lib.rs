pub use schedule::Schedule;

#[derive(Debug, Clone, Copy)]
pub enum Error {
    Empty,
    WrongNumberOfFields,
    InvalidMacro,
    Unknown,
}

mod schedule {
    use self::{
        iterator::{CopyRing, OwnedScheduleIter, ScheduleIter},
        table::FieldTable,
    };
    use crate::Error;
    use chrono::{DateTime, TimeZone, Utc};
    use std::str::FromStr;

    mod iterator;
    mod table;

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

        fn recalibrate<Tz: TimeZone + 'static>(&mut self, tz: &Tz) -> Option<DateTime<Tz>> {
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

        fn next<Tz: TimeZone + 'static>(&mut self, tz: &Tz) -> Option<DateTime<Tz>> {
            self.fields
                .next()
                .and_then(|dt| dt.and_local_timezone(tz.clone()).earliest())
        }

        fn new(fields: FieldTable) -> Self {
            Self {
                fields: Box::new(fields),
            }
        }

        pub fn hourly() -> Self {
            Self::new(hourly())
        }

        pub fn daily() -> Self {
            Self::new(daily())
        }

        pub fn weekly() -> Self {
            Self::new(weekly())
        }

        pub fn monthly() -> Self {
            Self::new(monthly())
        }

        pub fn yearly() -> Self {
            Self::new(annually())
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
                            'y' | 'a' => Ok(Schedule::yearly()),
                            'm' => Ok(Schedule::monthly()),
                            'w' => Ok(Schedule::weekly()),
                            'd' => Ok(Schedule::daily()),
                            'h' => Ok(Schedule::hourly()),
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
            .with_secs(CopyRing::from(0))
            .with_mins(CopyRing::from(0))
            .with_hours_iter(0..24)
            .with_days_month_iter(1..=31)
            .with_months_iter(1..=12)
            .build()
            .unwrap()
    }

    fn daily() -> FieldTable {
        todo!()
    }

    fn weekly() -> FieldTable {
        todo!()
    }

    fn annually() -> FieldTable {
        todo!()
    }

    fn monthly() -> FieldTable {
        todo!()
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
