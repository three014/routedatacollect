pub use collection::copy_ring::CopyRing;
pub use collection::iter::CycleIterMut;
pub use error::ParseError;
pub use schedule::Schedule;

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

mod error {
    use crate::table::BuildError;

    #[derive(Debug, Clone, Copy)]
    pub enum ParseError {
        Empty,
        WrongNumberOfFields,
        InvalidMacro,
        Unknown,
        BelowRange,
        AboveRange,
        DuplicateValue,
        IntervalWithoutWildcard,
        IntervalAndMultiple,
        Build(BuildError),
    }
}

mod collection;
mod table;
mod schedule {
    use self::iterator::{OwnedScheduleIter, ScheduleIter};
    use crate::table::FieldTable;
    use crate::{CopyRing, ParseError, DEFAULT_DAYS_MONTH, DEFAULT_HOURS, DEFAULT_MONTHS};
    use chrono::{DateTime, TimeZone};
    use std::str::FromStr;

    mod iterator;
    mod parser;

    #[derive(Clone, Debug)]
    pub struct Schedule {
        fields: Box<FieldTable>,
    }

    impl Schedule {
        pub fn after<Tz: TimeZone + 'static>(&mut self, when: &DateTime<Tz>) -> Option<DateTime<Tz>> {
            let first = self.fields.after(&when.naive_local());
            first.and_then(|dt| dt.and_local_timezone(when.timezone()).earliest())
        }

        pub fn into_iter_with_tz<Tz: TimeZone + 'static>(self, timezone: Tz) -> OwnedScheduleIter<Tz> {
            OwnedScheduleIter::new(self, timezone)
        }

        pub fn iter_with_tz<Tz: TimeZone + 'static>(&mut self, timezone: Tz) -> ScheduleIter<Tz> {
            ScheduleIter::new(self, timezone)
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
        type Err = ParseError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let fields: Vec<&str> = s.split_whitespace().collect();
            match fields.len() {
                0 => Err(ParseError::Empty),
                1 => {
                    let mut maybe_macro = fields[0].chars();
                    maybe_macro
                        .next()
                        .ok_or(ParseError::Unknown)?
                        .eq(&'@')
                        .then(|| match maybe_macro.next().ok_or(ParseError::InvalidMacro)? {
                            'y' | 'a' | 'Y' | 'A' => Ok(Schedule::yearly()),
                            'm' | 'M' => Ok(Schedule::monthly()),
                            'w' | 'W' => Ok(Schedule::weekly()),
                            'd' | 'D' => Ok(Schedule::daily()),
                            'h' | 'H' => Ok(Schedule::hourly()),
                            _ => Err(ParseError::InvalidMacro),
                        })
                        .unwrap_or(Err(ParseError::WrongNumberOfFields))
                }
                5 => unimplemented!("Will eventually be the equivalent of the 6-field version, but with '00' for seconds."),
                6 => FieldTable::try_from(fields).map(Schedule::new),
                _ => Err(ParseError::WrongNumberOfFields),
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
        FieldTable::builder()
            .with_secs(CopyRing::from(0))
            .with_mins(CopyRing::from(0))
            .with_hours(CopyRing::from(0))
            .with_days_month(CopyRing::from(0))
            .with_months(CopyRing::borrowed_with_size(&DEFAULT_MONTHS))
            .build()
            .unwrap()
    }

    #[cfg(test)]
    mod tests {
        use crate::Schedule;
        use chrono::Utc;
        use std::{
            sync::{Arc, Mutex},
            thread,
        };

        #[allow(unused)]
        fn foo() {
            let mut _s = Arc::new(Mutex::new(Schedule::hourly()));
            let _j = thread::spawn(move || {
                let mut _lock = _s.lock().unwrap();
            });
        }
    }
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

// TODO: Test this please
const fn fast_leap_year_check(year: u32) -> bool {
    !((year & 3) != 0 || (year & 15) != 0 && (year % 25) == 0)
}
