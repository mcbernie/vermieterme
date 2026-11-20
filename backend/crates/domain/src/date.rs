//! Calendar dates without time of day or time zone.
//!
//! Internally and in the web API dates are `YYYY-MM-DD` (decision 15); the tenant
//! app keeps receiving `YYYY-MM-DDT00:00:00.000Z`. Using a plain calendar date
//! avoids the time-zone shifts of JavaScript `Date` in the old code.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use time::{Date, Month};

/// Errors when reading a calendar date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DateError {
    /// Not in the form `YYYY-MM-DD` or not an existing day.
    #[error("Ungültiges Datum (erwartet JJJJ-MM-TT)")]
    Invalid,
}

/// A day in the Gregorian calendar between the years 1 and 9999.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalendarDate(Date);

impl CalendarDate {
    /// Builds a date from year, month (1–12) and day; fails for days that do not exist.
    pub fn from_ymd(year: i32, month: u8, day: u8) -> Result<Self, DateError> {
        if !(1..=9999).contains(&year) {
            return Err(DateError::Invalid);
        }
        let month = Month::try_from(month).map_err(|_| DateError::Invalid)?;
        Date::from_calendar_date(year, month, day)
            .map(CalendarDate)
            .map_err(|_| DateError::Invalid)
    }

    /// Parses exactly `YYYY-MM-DD` (four-digit year, two-digit month and day).
    pub fn parse(text: &str) -> Result<Self, DateError> {
        let bytes = text.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return Err(DateError::Invalid);
        }
        let number = |range: std::ops::Range<usize>| -> Result<u32, DateError> {
            let part = &bytes[range];
            if !part.iter().all(u8::is_ascii_digit) {
                return Err(DateError::Invalid);
            }
            Ok(part.iter().fold(0, |acc, b| acc * 10 + u32::from(b - b'0')))
        };
        let year = i32::try_from(number(0..4)?).map_err(|_| DateError::Invalid)?;
        let month = u8::try_from(number(5..7)?).map_err(|_| DateError::Invalid)?;
        let day = u8::try_from(number(8..10)?).map_err(|_| DateError::Invalid)?;
        Self::from_ymd(year, month, day)
    }

    /// Wraps a `time::Date`.
    pub fn from_date(date: Date) -> Self {
        CalendarDate(date)
    }

    /// The underlying `time::Date`.
    pub fn as_date(self) -> Date {
        self.0
    }

    /// The year.
    pub fn year(self) -> i32 {
        self.0.year()
    }

    /// The month, 1 = January.
    pub fn month(self) -> u8 {
        u8::from(self.0.month())
    }

    /// The day of the month.
    pub fn day(self) -> u8 {
        self.0.day()
    }

    /// Consecutive month number (`year * 12 + month - 1`); the difference of two
    /// indices is the number of calendar months between them.
    pub fn month_index(self) -> i64 {
        i64::from(self.year()) * 12 + i64::from(self.month()) - 1
    }

    /// The following day, e.g. the default start of a new prepayment after a period
    /// ends (`dayAfter` in the old code). `None` after 9999-12-31.
    pub fn next_day(self) -> Option<Self> {
        self.0
            .next_day()
            .filter(|d| d.year() <= 9999)
            .map(CalendarDate)
    }

    /// Timestamp form the tenant app expects: `YYYY-MM-DDT00:00:00.000Z`.
    pub fn to_app_timestamp(self) -> String {
        format!("{self}T00:00:00.000Z")
    }
}

impl fmt::Display for CalendarDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02}",
            self.year(),
            self.month(),
            self.day()
        )
    }
}

impl FromStr for CalendarDate {
    type Err = DateError;
    fn from_str(text: &str) -> Result<Self, DateError> {
        Self::parse(text)
    }
}

impl Serialize for CalendarDate {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for CalendarDate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}
