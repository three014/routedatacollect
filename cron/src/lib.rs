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
    use chrono::{DateTime, TimeZone, Utc};
    use std::str::FromStr;

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

    mod parser {
        use self::{
            days_of_the_month::DaysOfTheMonth, days_of_the_week::DaysOfTheWeek,
            hours::Hours, minutes::Minutes, months::Months, seconds::Seconds,
        };
        use crate::{
            table::{CronRing, FieldTable},
            CopyRing, ParseError,
        };
        use std::{collections::HashSet, str::FromStr};

        const COMMA: char = ',';
        const SLASH: char = '/';

        mod seconds {
            use super::SubExpr;

            pub struct Seconds(Vec<u32>);

            impl Seconds {
                const MIN: u32 = 0;
                const MAX: u32 = 59;
            }

            impl From<Seconds> for Vec<u32> {
                fn from(value: Seconds) -> Self {
                    value.0
                }
            }

            impl From<Vec<u32>> for Seconds {
                fn from(value: Vec<u32>) -> Self {
                    Self(value)
                }
            }

            impl SubExpr for Seconds {
                fn min() -> u32 {
                    Seconds::MIN
                }

                fn max() -> u32 {
                    Seconds::MAX
                }

                fn values(&self) -> &Vec<u32> {
                    &self.0
                }
            }
        }

        mod minutes {
            use super::SubExpr;

            pub struct Minutes(Vec<u32>);

            impl Minutes {
                const MIN: u32 = 0;
                const MAX: u32 = 59;
            }

            impl From<Minutes> for Vec<u32> {
                fn from(value: Minutes) -> Self {
                    value.0
                }
            }

            impl From<Vec<u32>> for Minutes {
                fn from(value: Vec<u32>) -> Self {
                    Self(value)
                }
            }

            impl SubExpr for Minutes {
                fn min() -> u32 {
                    Minutes::MIN
                }

                fn max() -> u32 {
                    Minutes::MAX
                }

                fn values(&self) -> &Vec<u32> {
                    &self.0
                }
            }
        }

        mod hours {
            use super::SubExpr;

            pub struct Hours(Vec<u32>);

            impl Hours {
                const MIN: u32 = 0;
                const MAX: u32 = 23;
            }

            impl From<Hours> for Vec<u32> {
                fn from(value: Hours) -> Self {
                    value.0
                }
            }

            impl From<Vec<u32>> for Hours {
                fn from(value: Vec<u32>) -> Self {
                    Self(value)
                }
            }

            impl SubExpr for Hours {
                fn min() -> u32 {
                    Hours::MIN
                }

                fn max() -> u32 {
                    Hours::MAX
                }

                fn values(&self) -> &Vec<u32> {
                    &self.0
                }
            }
        }

        mod days_of_the_month {
            use super::SubExpr;

            pub struct DaysOfTheMonth(Vec<u32>);

            impl DaysOfTheMonth {
                const MIN: u32 = 1;
                const MAX: u32 = 31;
            }

            impl From<DaysOfTheMonth> for Vec<u32> {
                fn from(value: DaysOfTheMonth) -> Self {
                    value.0
                }
            }

            impl From<Vec<u32>> for DaysOfTheMonth {
                fn from(value: Vec<u32>) -> Self {
                    Self(value)
                }
            }

            impl SubExpr for DaysOfTheMonth {
                fn min() -> u32 {
                    DaysOfTheMonth::MIN
                }

                fn max() -> u32 {
                    DaysOfTheMonth::MAX
                }

                fn values(&self) -> &Vec<u32> {
                    &self.0
                }
            }
        }

        mod months {
            use super::SubExpr;

            pub struct Months(Vec<u32>);

            impl Months {
                const MIN: u32 = 1;
                const MAX: u32 = 12;
            }

            impl From<Months> for Vec<u32> {
                fn from(value: Months) -> Self {
                    value.0
                }
            }

            impl From<Vec<u32>> for Months {
                fn from(value: Vec<u32>) -> Self {
                    Self(value)
                }
            }

            impl SubExpr for Months {
                fn min() -> u32 {
                    Months::MIN
                }

                fn max() -> u32 {
                    Months::MAX
                }

                fn values(&self) -> &Vec<u32> {
                    &self.0
                }
            }
        }

        mod days_of_the_week {
            use super::SubExpr;

            pub struct DaysOfTheWeek(Vec<u32>);

            impl DaysOfTheWeek {
                const MIN: u32 = 0;
                const MAX: u32 = 6;
            }

            impl From<DaysOfTheWeek> for Vec<u32> {
                fn from(value: DaysOfTheWeek) -> Self {
                    value.0
                }
            }

            impl From<Vec<u32>> for DaysOfTheWeek {
                fn from(value: Vec<u32>) -> Self {
                    Self(value)
                }
            }

            impl SubExpr for DaysOfTheWeek {
                fn min() -> u32 {
                    DaysOfTheWeek::MIN
                }

                fn max() -> u32 {
                    DaysOfTheWeek::MAX
                }

                fn values(&self) -> &Vec<u32> {
                    &self.0
                }
            }
        }

        trait SubExpr: Sized + Into<Vec<u32>> {
            /// Returns the minimum value allowed for
            /// this subexpression.
            fn min() -> u32;

            /// Returns the maximum value allowed
            /// for this subexpression.
            fn max() -> u32;

            /// Returns a reference to the set of 
            /// values given by the user.
            fn values(&self) -> &Vec<u32>;
            
            /// Validates the values, and on success
            /// returns a `CopyRing<u8>` of the values
            /// for use in a `FieldTable`.
            fn validate(self, dupck: &mut HashSet<u32>) -> Result<CronRing, ParseError> {
                dupck.clear();
                let fields = self.values();
                let min = <Self as SubExpr>::min();
                let max = <Self as SubExpr>::max();
                if fields.is_empty() {
                    return Err(ParseError::Empty);
                }
                if fields.iter().any(|&f| f < min) {
                    return Err(ParseError::BelowRange);
                }
                if fields.iter().any(|&f| f > max) {
                    return Err(ParseError::AboveRange);
                }
                if fields.iter().any(|&f| !dupck.insert(f)) {
                    return Err(ParseError::DuplicateValue);
                }

                Ok(CopyRing::from_iter(
                    self.into().into_iter().map(|x| x as u8),
                ))
            }
        }

        struct CronSubExpr<S>(S)
        where
            S: SubExpr + From<Vec<u32>>;

        impl<S> CronSubExpr<S>
        where
            S: SubExpr + From<Vec<u32>>,
        {
            pub fn validate(self, dupck: &mut HashSet<u32>) -> Result<CronRing, ParseError> {
                self.0.validate(dupck)
            }
        }

        impl<S> FromStr for CronSubExpr<S>
        where
            S: SubExpr + From<Vec<u32>>,
        {
            type Err = ParseError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let contains_multiple = s.chars().any(|c| c.eq(&COMMA));
                let contains_interval = s.chars().any(|c| c.eq(&SLASH));
                match (contains_multiple, contains_interval) {
                    (true, true) => Err(ParseError::IntervalAndMultiple),
                    (true, false) => todo!(),
                    (false, true) => todo!(),
                    (false, false) => todo!(),
                }
            }
        }

        impl TryFrom<Vec<&str>> for FieldTable {
            type Error = ParseError;

            fn try_from(value: Vec<&str>) -> Result<Self, Self::Error> {
                let mut dupck = HashSet::with_capacity(16);
                let seconds =
                    CronSubExpr::<Seconds>::from_str(value[0])?.validate(&mut dupck)?;
                let minutes =
                    CronSubExpr::<Minutes>::from_str(value[1])?.validate(&mut dupck)?;
                let hours = CronSubExpr::<Hours>::from_str(value[2])?.validate(&mut dupck)?;
                let days_of_the_month =
                    CronSubExpr::<DaysOfTheMonth>::from_str(value[3])?.validate(&mut dupck)?;
                let months = CronSubExpr::<Months>::from_str(value[4])?.validate(&mut dupck)?;
                let days_of_the_week =
                    CronSubExpr::<DaysOfTheWeek>::from_str(value[5])?.validate(&mut dupck)?;
                Self::builder()
                    .with_secs(seconds)
                    .with_mins(minutes)
                    .with_hours(hours)
                    .with_days_month(days_of_the_month)
                    .with_months(months)
                    .with_days_week(days_of_the_week)
                    .build()
                    .map_err(ParseError::Build)
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

        #[allow(unused)]
        fn foo() {
            let mut _s = Arc::new(Mutex::new(Schedule::hourly()));
            let _j = thread::spawn(move || {
                let mut lock = _s.lock().unwrap();
                let _iter = lock.iter_with_timezone(Utc);
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
