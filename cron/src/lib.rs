pub use schedule::CopyRing;
pub use schedule::Schedule;

#[derive(Debug, Clone, Copy)]
pub enum Error {
    Empty,
    WrongNumberOfFields,
    InvalidMacro,
    Unknown,
}

static DEFAULT_SECONDS: [u8; 60] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59,
];
static DEFAULT_MINUTES: [u8; 60] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59,
];
static DEFAULT_HOURS: [u8; 24] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
];
static DEFAULT_DAYS_WEEK: [u8; 7] = [0, 1, 2, 3, 4, 5, 6];
static DEFAULT_DAYS_MONTH: [u8; 31] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31,
];
static DEFAULT_MONTHS: [u8; 12] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
const MONTH_TO_DAYS_NO_LEAP: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

mod schedule {
    use self::{
        iterator::{OwnedScheduleIter, ScheduleIter},
        table::FieldTable,
    };
    use crate::{Error, DEFAULT_DAYS_MONTH, DEFAULT_HOURS, DEFAULT_MONTHS};
    use chrono::{DateTime, TimeZone, Utc};
    pub use iterator::CopyRing;
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
            .with_hours(CopyRing::borrowed_with_size(&DEFAULT_HOURS))
            .with_days_month(CopyRing::borrowed_with_size(&DEFAULT_DAYS_MONTH))
            .with_months(CopyRing::borrowed_with_size(&DEFAULT_MONTHS))
            .build()
            .unwrap()
    }

    fn daily() -> FieldTable {
        FieldTable::builder()
            .with_secs(CopyRing::from(0))
            .with_mins(CopyRing::from(0))
            .with_hours(CopyRing::from(0))
            .with_days_month(CopyRing::borrowed_with_size(&DEFAULT_DAYS_MONTH))
            .with_months(CopyRing::borrowed_with_size(&DEFAULT_MONTHS))
            .build()
            .unwrap()
    }

    fn weekly() -> FieldTable {
        FieldTable::builder()
            .with_secs(CopyRing::from(0))
            .with_mins(CopyRing::from(0))
            .with_hours(CopyRing::from(0))
            .with_days_week(CopyRing::from(0))
            .with_months(CopyRing::borrowed_with_size(&DEFAULT_MONTHS))
            .build()
            .unwrap()
    }

    fn annually() -> FieldTable {
        FieldTable::builder()
            .with_secs(CopyRing::from(0))
            .with_mins(CopyRing::from(0))
            .with_hours(CopyRing::from(0))
            .with_days_month(CopyRing::from(0))
            .with_months(CopyRing::from(0))
            .build()
            .unwrap()
    }

    fn monthly() -> FieldTable {
        todo!()
    }

    #[cfg(test)]
    mod tests {
        use crate::Schedule;
        use chrono::Utc;
        use std::{
            sync::{Arc, Mutex},
            thread,
        };

        fn foo() {
            let mut _s = Arc::new(Mutex::new(Schedule::hourly()));
            let _j = thread::spawn(move || {
                let mut lock = _s.lock().unwrap();
                let _iter = lock.iter_with_timezone(Utc);
            });
        }
    }
}

const fn days_in_a_month(month: u8, year: u32) -> u8 {
    assert!(
        month >= 1 && month <= 12,
        "Number has to be from 1 - 12, corresponding to the months of the year."
    );
    let month_to_days_with_leap: [u8; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let converter = [MONTH_TO_DAYS_NO_LEAP, month_to_days_with_leap];
    converter[is_leap_year(year) as usize][month as usize - 1]
}

const fn is_leap_year(year: u32) -> bool {
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
