use chrono::{DateTime, NaiveDateTime, TimeZone};

use self::fields::{DateBuilder, TimeBuilder};

#[derive(Clone, Copy, Debug)]
pub enum Error {
    MissingField,
    EmptyRing,
    OutOfRange,
}

impl std::fmt::Display for self::Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MissingField => writeln!(f, "an inner field was missing"),
            Error::EmptyRing => writeln!(
                f,
                "all fields were supplied, but at least one buffer was empty"
            ),
            Error::OutOfRange => writeln!(
                f,
                "at least one buffer contained values that were out of range for that field"
            ),
        }
    }
}
impl std::error::Error for self::Error {}

#[derive(Clone, Copy, Debug)]
pub struct BuildError {
    time: Option<Error>,
    date: Option<Error>,
}
impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.time, self.date) {
            (None, None) => writeln!(f, "no errors reported"),
            (None, Some(date)) => writeln!(f, "date error: {}", date),
            (Some(time), None) => writeln!(f, "time error: {}", time),
            (Some(time), Some(date)) => writeln!(f, "date error: {}, time error: {}", date, time),
        }
    }
}

pub trait FieldTableBuilder {}

mod fields;

#[derive(Clone, Debug)]
pub struct FieldTable {}

/// A builder pattern for the `FieldTable`.
/// To build a `FieldTable`, supply the builder
/// with values for each of fields.
///
/// Each method accepts an item that impl's
/// `Into<CopyRing<u8>>`, which is any of the
/// following:
/// - a single value `u8`
/// - any struct that impl's `IntoIterator<Item = u8>`
/// - a `Vec<u8>` itself
///     
/// The only field that behaves differently is the `Day` field.
/// For the days, choose one of the
/// methods `with_days_of_the_month_only`,
/// `with_days_of_the_week_only`, or `with_days_of_both`.
/// If the user gave a '*' for both the days of weeks/days of month
/// then `with_days_of_the_month_only` should be used and
/// a full iterator of 1..=31 should be supplied.
///
/// # Failure
///
/// All fields should be supplied, or the build fails
/// with an `Error::MissingField`.
/// Each field should contain values that are
/// within bounds for that field, or else
/// the build will fail with an `Error::OutOfRange`.
/// Likewise, a field should not be empty, or else
/// the build will fail with an `Error::EmptyRing`.
///
/// # Assumptions
///
/// The main assumption that will not be tested is that
/// the supplied field values are sorted, and that
/// the first value is the lowest in the range.
#[derive(Default)]
pub struct Builder {
    time: TimeBuilder,
    date: DateBuilder,
}

impl Builder {
    pub fn build(&mut self) -> Result<FieldTable, [Option<Error>; 2]> {
        todo!()
    }
}

impl FieldTable {
    pub fn after<Tz: TimeZone + 'static>(
        &mut self,
        date_time: &DateTime<Tz>,
    ) -> Option<NaiveDateTime> {
        // let (sec, overflow) = self.secs.first_after(date_time.second() as u8);
        // let (min, overflow) = self.mins.first_after(date_time.minute() as u8, overflow);
        // let (hour, overflow) = self.hours.first_after(date_time.hour() as u8, overflow);
        // let (day, overflow) = self.days.first_after(
        //     date_time.day() as u8,
        //     date_time.weekday().num_days_from_sunday() as u8,
        //     overflow,
        //     date_time.month() as u8,
        //     date_time.year() as u32,
        // );
        // let (month, overflow) = self.months.first_after(overflow, date_time.month() as u8);
        // let year = date_time.year() + overflow as i32;
        //
        // NaiveDate::from_ymd_opt(year, month as u32, day as u32)
        //     .and_then(|date| date.and_hms_opt(hour as u32, min as u32, sec as u32))
        todo!()
    }

    pub fn next(&mut self, curr_month: u8, curr_year: u32) -> Option<NaiveDateTime> {
        // let (sec, overflow) = self.secs.next();
        // let (min, overflow) = self.mins.next(overflow);
        // let (hour, overflow) = self.hours.next(overflow);
        // let (day, overflow) = self.days.next(overflow, curr_month, curr_year);
        // let (month, overflow) = self.months.next(overflow);
        // let year = curr_year + overflow as u32;
        //
        // NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)
        //     .and_then(|date| date.and_hms_opt(hour as u32, min as u32, sec as u32))
        todo!()
    }

    pub fn builder() -> Builder {
        Builder::default()
    }
}

#[cfg(test)]
mod test;
