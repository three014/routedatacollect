use self::inner::{Days, Hours, Minutes, Months, Seconds};
use super::iterator::CopyRing;
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, TimeZone, Timelike};

#[derive(Clone, Debug)]
pub enum Error {
    MissingField,
    EmptyRing,
    OutOfRange,
}

mod inner;

#[derive(Clone, Debug)]
pub struct FieldTable {
    secs: Seconds,
    mins: Minutes,
    hours: Hours,
    days: Days,
    months: Months,
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
    secs: Option<CopyRing<u8>>,
    mins: Option<CopyRing<u8>>,
    hrs: Option<CopyRing<u8>>,
    days: Option<Days>,
    months: Option<CopyRing<u8>>,
}

impl Builder {
    pub fn with_secs_iter(&mut self, secs: impl IntoIterator<Item = u8>) -> &mut Self {
        self.secs = Some(CopyRing::from_iter(secs));
        self
    }

    pub fn with_secs(&mut self, secs: impl Into<CopyRing<u8>>) -> &mut Self {
        self.secs = Some(secs.into());
        self
    }

    pub fn with_mins_iter(&mut self, mins: impl IntoIterator<Item = u8>) -> &mut Self {
        self.mins = Some(CopyRing::from_iter(mins));
        self
    }

    pub fn with_mins(&mut self, mins: impl Into<CopyRing<u8>>) -> &mut Self {
        self.mins = Some(mins.into());
        self
    }

    pub fn with_hrs_iter(&mut self, hrs: impl IntoIterator<Item = u8>) -> &mut Self {
        self.hrs = Some(CopyRing::from_iter(hrs));
        self
    }

    pub fn with_hrs(&mut self, hrs: impl Into<CopyRing<u8>>) -> &mut Self {
        self.mins = Some(hrs.into());
        self
    }

    pub fn with_days_of_the_month_only_iter(
        &mut self,
        days: impl IntoIterator<Item = u8>,
    ) -> &mut Self {
        self.days = Some(Days::Month(CopyRing::from_iter(days)));
        self
    }

    pub fn with_days_of_the_month_only(&mut self, days: impl Into<CopyRing<u8>>) -> &mut Self {
        self.days = Some(Days::Month(days.into()));
        self
    }

    pub fn with_days_of_the_week_only_iter(
        &mut self,
        days: impl IntoIterator<Item = u8>,
    ) -> &mut Self {
        self.days = Some(Days::Week((CopyRing::from_iter(days), Default::default())));
        self
    }

    pub fn with_days_of_the_week_only(&mut self, days: impl Into<CopyRing<u8>>) -> &mut Self {
        self.days = Some(Days::Week((days.into(), Default::default())));
        self
    }

    pub fn with_days_of_both_iter(
        &mut self,
        week: impl IntoIterator<Item = u8>,
        month: impl IntoIterator<Item = u8>,
    ) -> &mut Self {
        self.days = Some(Days::Both {
            month: CopyRing::from_iter(month),
            week: (CopyRing::from_iter(week), Default::default()),
        });
        self
    }

    pub fn with_days_of_both(
        &mut self,
        week: impl Into<CopyRing<u8>>,
        month: impl Into<CopyRing<u8>>,
    ) -> &mut Self {
        self.days = Some(Days::Both {
            month: month.into(),
            week: (week.into(), Default::default()),
        });
        self
    }

    pub fn with_months_iter(&mut self, months: impl IntoIterator<Item = u8>) -> &mut Self {
        self.months = Some(CopyRing::from_iter(months));
        self
    }

    pub fn with_months(&mut self, months: impl Into<CopyRing<u8>>) -> &mut Self {
        self.mins = Some(months.into());
        self
    }

    pub fn build(&mut self) -> Result<FieldTable, Error> {
        if self.secs.is_none()
            || self.mins.is_none()
            || self.hrs.is_none()
            || self.days.is_none()
            || self.months.is_none()
        {
            return Err(Error::MissingField);
        }

        let secs = self.secs.take().unwrap();
        let mins = self.mins.take().unwrap();
        let hrs = self.hrs.take().unwrap();
        let days = self.days.take().unwrap();
        let months = self.months.take().unwrap();

        if secs.is_empty()
            || mins.is_empty()
            || hrs.is_empty()
            || match days {
                Days::Both {
                    ref month,
                    ref week,
                } => month.is_empty() || week.0.is_empty(),
                Days::Month(ref month) => month.is_empty(),
                Days::Week(ref week) => week.0.is_empty(),
            }
            || months.is_empty()
        {
            return Err(Error::EmptyRing);
        }

        if secs.last().unwrap() >= 60
            || mins.last().unwrap() >= 60
            || hrs.last().unwrap() >= 24
            || match days {
                Days::Both {
                    ref month,
                    ref week,
                } => {
                    month.last().unwrap() > 31
                        || month.first().unwrap() < 1
                        || week.0.last().unwrap() >= 7
                }
                Days::Month(ref month) => month.last().unwrap() > 31 || month.first().unwrap() < 1,
                Days::Week(ref week) => week.0.last().unwrap() >= 7,
            }
            || months.last().unwrap() > 12
            || months.first().unwrap() < 1
        {
            return Err(Error::OutOfRange);
        }

        Ok(FieldTable {
            secs: Seconds::new(secs),
            mins: Minutes::new(mins),
            hours: Hours::new(hrs),
            days,
            months: Months::new(months),
        })
    }
}

impl FieldTable {
    pub fn after<Tz: TimeZone + 'static>(
        &mut self,
        date_time: &DateTime<Tz>,
    ) -> Option<NaiveDateTime> {
        let (sec, overflow) = self.secs.first_after(date_time.second() as u8);
        let (min, overflow) = self.mins.first_after(date_time.minute() as u8, overflow);
        let (hour, overflow) = self.hours.first_after(date_time.hour() as u8, overflow);
        let (day, overflow) = self.days.first_after(
            date_time.day() as u8,
            date_time.weekday().num_days_from_sunday() as u8,
            overflow,
            date_time.month() as u8,
            date_time.year() as u32,
        );
        let (month, overflow) = self.months.first_after(overflow, date_time.month() as u8);
        let year = date_time.year() + overflow as i32;

        NaiveDate::from_ymd_opt(year, month as u32, day as u32)
            .and_then(|date| date.and_hms_opt(hour as u32, min as u32, sec as u32))
    }

    pub fn next(&mut self, curr_month: u8, curr_year: u32) -> Option<NaiveDateTime> {
        let (sec, overflow) = self.secs.next();
        let (min, overflow) = self.mins.next(overflow);
        let (hour, overflow) = self.hours.next(overflow);
        let (day, overflow) = self.days.next(overflow, curr_month, curr_year);
        let (month, overflow) = self.months.next(overflow);
        let year = curr_year + overflow as u32;

        NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)
            .and_then(|date| date.and_hms_opt(hour as u32, min as u32, sec as u32))
    }

    pub fn builder() -> Builder {
        Builder::default()
    }
}

#[cfg(test)]
mod test;
