use crate::CopyRing;

use self::fields::{Date, DateBuilder, Time, TimeBuilder};
use chrono::{DateTime, Datelike, NaiveDateTime, TimeZone, Timelike};

pub type CronRing = CopyRing<'static, u8, 1>;

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

mod fields;

#[derive(Clone, Debug)]
pub struct FieldTable {
    time: Time,
    date: Date,
}

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
    pub fn with_secs_iter(&mut self, secs: impl IntoIterator<Item = u8>) -> &mut Self {
        self.time.with_secs_iter(secs);
        self
    }

    pub fn with_secs(&mut self, secs: CronRing) -> &mut Self {
        self.time.with_secs(secs);
        self
    }

    pub fn with_mins_iter(&mut self, mins: impl IntoIterator<Item = u8>) -> &mut Self {
        self.time.with_mins_iter(mins);
        self
    }

    pub fn with_mins(&mut self, mins: CronRing) -> &mut Self {
        self.time.with_mins(mins);
        self
    }

    pub fn with_hours_iter(&mut self, hours: impl IntoIterator<Item = u8>) -> &mut Self {
        self.time.with_hours_iter(hours);
        self
    }

    pub fn with_hours(&mut self, hours: CronRing) -> &mut Self {
        self.time.with_hours(hours);
        self
    }

    pub fn with_days_week(&mut self, weekdays: CronRing) -> &mut Self {
        self.date.with_days_week(weekdays);
        self
    }

    pub fn with_days_week_iter(&mut self, weekdays: impl IntoIterator<Item = u8>) -> &mut Self {
        self.date.with_days_week_iter(weekdays);
        self
    }

    pub fn with_days_month(&mut self, month_days: CronRing) -> &mut Self {
        self.date.with_days_month(month_days);
        self
    }

    pub fn with_days_month_iter(&mut self, month_days: impl IntoIterator<Item = u8>) -> &mut Self {
        self.date.with_days_month_iter(month_days);
        self
    }

    pub fn with_months_iter(&mut self, months: impl IntoIterator<Item = u8>) -> &mut Self {
        self.date.with_months_iter(months);
        self
    }

    pub fn with_months(&mut self, months: CronRing) -> &mut Self {
        self.date.with_months(months);
        self
    }

    pub fn build(&mut self) -> Result<FieldTable, BuildError> {
        let time = self.time.build();
        let date = self.date.build();
        match (time, date) {
            (Ok(time), Ok(date)) => Ok(FieldTable { time, date }),
            (Ok(_), Err(d)) => Err(BuildError {
                time: None,
                date: Some(d),
            }),
            (Err(t), Ok(_)) => Err(BuildError {
                time: Some(t),
                date: None,
            }),
            (Err(t), Err(d)) => Err(BuildError {
                time: Some(t),
                date: Some(d),
            }),
        }
    }
}

impl FieldTable {
    pub fn after<Tz: TimeZone + 'static>(
        &mut self,
        date_time: &DateTime<Tz>,
    ) -> Option<NaiveDateTime> {
        let (time, overflow) = self.time.first_after(
            date_time.second() as u8,
            date_time.minute() as u8,
            date_time.hour() as u8,
        )?;
        let date = self.date.first_after(
            overflow,
            date_time.day() as u8,
            date_time.weekday().num_days_from_sunday() as u8,
            date_time.month() as u8,
            date_time.year() as u32,
        )?;

        Some(date.and_time(time))
        // NaiveDate::from_ymd_opt(year, month as u32, day as u32)
        //     .and_then(|date| date.and_hms_opt(hour as u32, min as u32, sec as u32))
    }

    pub fn next(&mut self) -> Option<NaiveDateTime> {
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
