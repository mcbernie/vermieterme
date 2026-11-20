//! Consumer price index (Verbraucherpreisindex, VPI) for index rents.
//!
//! Values are tenths as integers (`1196` = 119.6), as published by the Bundesbank
//! and as in the API (`valueTenths`). Parsing the Bundesbank response and the
//! rent adjustment proposal live in the submodules.

mod adjustment;
mod bundesbank;

use serde::{Deserialize, Serialize};

pub use adjustment::{IndexAdjustment, index_adjustment, months_since};
pub use bundesbank::{VpiDataPoint, VpiParseError, parse_bundesbank_response};

use crate::decimal::parse_scaled;

/// An index value in tenths of a point.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct VpiValue(i64);

impl VpiValue {
    /// Wraps a value in tenths (`1196` = 119.6).
    pub const fn from_tenths(tenths: i64) -> Self {
        VpiValue(tenths)
    }

    /// The value in tenths.
    pub const fn tenths(self) -> i64 {
        self.0
    }

    /// Parses a decimal text (`"119.35"`), rounded half away from zero to one decimal.
    pub fn parse(text: &str) -> Option<Self> {
        parse_scaled(text, 1)
            .ok()
            .and_then(|t| i64::try_from(t).ok())
            .map(VpiValue)
    }

    /// Converts a float value (legacy database, backup format 1) via its shortest
    /// decimal representation, rounded to one decimal.
    pub fn from_f64(value: f64) -> Option<Self> {
        if value.is_finite() {
            Self::parse(&value.to_string())
        } else {
            None
        }
    }
}

/// Bundesbank series code of a supported base year (`2020` → `I20`).
pub fn base_year_code(base_year: i32) -> Option<&'static str> {
    match base_year {
        2020 => Some("I20"),
        2015 => Some("I15"),
        2010 => Some("I10"),
        _ => None,
    }
}

/// A stored index value of one month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VpiEntry {
    /// Year of the observation.
    pub year: i32,
    /// Month of the observation, 1–12.
    pub month: u8,
    /// Index value.
    pub value: VpiValue,
    /// Base year of the series (2020 = 100).
    pub base_year: i32,
}

/// The most recent entry of a base year, i.e. the "current VPI" of a proposal.
pub fn latest_for_base_year(entries: &[VpiEntry], base_year: i32) -> Option<&VpiEntry> {
    entries
        .iter()
        .filter(|e| e.base_year == base_year)
        .max_by_key(|e| (e.year, e.month))
}
