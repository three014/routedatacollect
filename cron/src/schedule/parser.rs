use self::{
    days_of_the_month::DaysOfTheMonth, days_of_the_week::DaysOfTheWeek, hours::Hours,
    minutes::Minutes, months::Months, seconds::Seconds, tokens::TokenStream,
};
use crate::{
    table::{CronRing, FieldTable},
    CopyRing, ParseError,
};
use std::collections::HashSet;

mod tokens;
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
        fn as_strs() -> Option<&'static [&'static str]> {
            None
        }

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
        fn as_strs() -> Option<&'static [&'static str]> {
            None
        }

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
        fn as_strs() -> Option<&'static [&'static str]> {
            None
        }

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
        fn as_strs() -> Option<&'static [&'static str]> {
            None
        }

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
        const fn get() -> &'static [&'static str] {
            &[
                "INV", "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV",
                "DEC",
            ]
        }
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
        fn as_strs() -> Option<&'static [&'static str]> {
            Some(Months::get())
        }

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
        const fn get() -> &'static [&'static str] {
            &["SUN", "MON", "TUES", "WED", "THURS", "FRI", "SAT"]
        }
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
        fn as_strs() -> Option<&'static [&'static str]> {
            Some(DaysOfTheWeek::get())
        }

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

trait SubExpr: Into<Vec<u32>> {
    /// Returns the strings that
    /// correspond to the allowed keywords
    /// that a user can type in-place of
    /// the number values.
    fn as_strs() -> Option<&'static [&'static str]>;

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
        let values = self.values();
        let min = <Self as SubExpr>::min();
        let max = <Self as SubExpr>::max();
        if values.is_empty() {
            return Err(ParseError::Empty);
        }
        if values.iter().any(|&f| f < min) {
            return Err(ParseError::BelowRange);
        }
        if values.iter().any(|&f| f > max) {
            return Err(ParseError::AboveRange);
        }
        if values.iter().any(|&f| !dupck.insert(f)) {
            return Err(ParseError::DuplicateValue);
        }

        let mut values: Vec<u32> = self.into();
        values.sort_unstable();

        Ok(CopyRing::from_iter(values.into_iter().map(|x| x as u8)))
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

    pub fn parse_str(s: &str) -> Result<Self, ParseError> {
        let _token_stream = TokenStream::tokenize(s.char_indices())?;

        todo!()
    }
}

impl TryFrom<Vec<&str>> for FieldTable {
    type Error = ParseError;

    fn try_from(value: Vec<&str>) -> Result<Self, Self::Error> {
        let mut dupck = HashSet::with_capacity(16);
        let seconds = CronSubExpr::<Seconds>::parse_str(value[0])?.validate(&mut dupck)?;
        let minutes = CronSubExpr::<Minutes>::parse_str(value[1])?.validate(&mut dupck)?;
        let hours = CronSubExpr::<Hours>::parse_str(value[2])?.validate(&mut dupck)?;
        let days_of_the_month =
            CronSubExpr::<DaysOfTheMonth>::parse_str(value[3])?.validate(&mut dupck)?;
        let months = CronSubExpr::<Months>::parse_str(value[4])?.validate(&mut dupck)?;
        let days_of_the_week =
            CronSubExpr::<DaysOfTheWeek>::parse_str(value[5])?.validate(&mut dupck)?;
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
