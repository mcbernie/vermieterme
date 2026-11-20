//! Money as an integer number of cents.
//!
//! Every amount in the backend is a [`Cents`] (decision 3 in the plan). Shares are
//! computed with exact integer arithmetic and rounded half away from zero; `f64`
//! appears only when converting at the edges (legacy import, euro numbers in the
//! tenant-app JSON), never in arithmetic.

use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

use serde::{Deserialize, Serialize};

use crate::decimal::{DecimalError, parse_scaled};
use crate::rounding::{div_round_half_away, saturate_i64};

/// Errors when turning external input into [`Cents`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MoneyError {
    /// The text or number is not a valid amount.
    #[error("Ungültiger Betrag")]
    Invalid,
    /// The amount is outside [`Cents::LIMIT`].
    #[error("Betrag ist zu groß")]
    OutOfRange,
}

/// An amount of money in euro cents (negative = credit / refund).
///
/// Arithmetic saturates at the `i64` bounds instead of wrapping or panicking; inputs
/// are bounded by [`Cents::LIMIT`], so saturation never happens with valid data.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Cents(i64);

impl Cents {
    /// Zero cents.
    pub const ZERO: Cents = Cents(0);

    /// Largest accepted absolute amount for input (10 billion euros). Keeping inputs
    /// below this bound means sums over any realistic number of positions cannot
    /// overflow `i64`.
    pub const LIMIT: i64 = 1_000_000_000_000;

    /// Wraps a number of cents.
    pub const fn new(cents: i64) -> Self {
        Cents(cents)
    }

    /// The number of cents.
    pub const fn value(self) -> i64 {
        self.0
    }

    /// Whether the absolute amount is within [`Cents::LIMIT`]; the API uses this to
    /// validate incoming cent values.
    pub const fn is_within_limit(self) -> bool {
        self.0 >= -Self::LIMIT && self.0 <= Self::LIMIT
    }

    /// Parses a euro amount written as a decimal number with `.` as separator
    /// (`"1234.5"`, `"-0.05"`). More than two decimals are rounded half away from zero.
    pub fn parse_euros(text: &str) -> Result<Self, MoneyError> {
        let cents = parse_scaled(text, 2).map_err(|e| match e {
            DecimalError::Malformed => MoneyError::Invalid,
            DecimalError::TooLarge => MoneyError::OutOfRange,
        })?;
        Self::checked_from_i128(cents)
    }

    /// Converts a euro amount stored as a float (legacy database, backup format 1).
    ///
    /// The shortest decimal representation of the float is rounded, so `0.1 + 0.2`
    /// stored as `0.30000000000000004` becomes 30 cents and `1.005` becomes 101 cents
    /// (a plain `(x * 100).round()` would give 100). Huge values give
    /// [`MoneyError::OutOfRange`], values below half a cent give zero; only NaN and
    /// infinity are [`MoneyError::Invalid`].
    pub fn from_euros_f64(euros: f64) -> Result<Self, MoneyError> {
        if !euros.is_finite() {
            return Err(MoneyError::Invalid);
        }
        // `Display` for f64 prints the shortest round-trip decimal without exponent.
        Self::parse_euros(&euros.to_string())
    }

    /// The amount as a euro float, only for JSON consumers that expect euro numbers
    /// (tenant app, PDF renderer). Never use the result for further arithmetic.
    pub fn to_euros_f64(self) -> f64 {
        // Exact for every amount within LIMIT (well below 2^53).
        self.0 as f64 / 100.0
    }

    /// Multiplies by `numerator / denominator` and rounds half away from zero.
    ///
    /// Returns `None` when `denominator` is zero.
    pub fn mul_ratio(self, numerator: i64, denominator: i64) -> Option<Self> {
        let product = i128::from(self.0) * i128::from(numerator);
        div_round_half_away(product, i128::from(denominator)).map(|c| Cents(saturate_i64(c)))
    }

    /// Multiplies by a whole factor (e.g. a monthly amount by a number of months).
    pub fn times(self, factor: i64) -> Self {
        Cents(saturate_i64(i128::from(self.0) * i128::from(factor)))
    }

    /// German display without currency sign: `-1.234,56`.
    pub fn format_de(self) -> String {
        let abs = self.0.unsigned_abs();
        let euros = (abs / 100).to_string();
        let mut grouped = String::with_capacity(euros.len() + euros.len() / 3);
        for (i, digit) in euros.chars().enumerate() {
            if i > 0 && (euros.len() - i).is_multiple_of(3) {
                grouped.push('.');
            }
            grouped.push(digit);
        }
        let sign = if self.0 < 0 { "-" } else { "" };
        format!("{sign}{grouped},{:02}", abs % 100)
    }

    /// German currency display like `Intl.NumberFormat("de-DE", EUR)`: `1.234,56 €`
    /// (no-break space before the sign).
    pub fn format_eur(self) -> String {
        format!("{}\u{a0}€", self.format_de())
    }

    /// Range-checks a parsed value.
    fn checked_from_i128(cents: i128) -> Result<Self, MoneyError> {
        let cents = i64::try_from(cents).map_err(|_| MoneyError::OutOfRange)?;
        let value = Cents(cents);
        if value.is_within_limit() {
            Ok(value)
        } else {
            Err(MoneyError::OutOfRange)
        }
    }
}

/// Plain decimal euro notation with a dot (`1234.56`, `-0.05`), e.g. for logs of
/// totals in tests. Use [`Cents::format_de`] for documents.
impl fmt::Display for Cents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let abs = self.0.unsigned_abs();
        let sign = if self.0 < 0 { "-" } else { "" };
        write!(f, "{sign}{}.{:02}", abs / 100, abs % 100)
    }
}

impl Add for Cents {
    type Output = Cents;
    fn add(self, rhs: Cents) -> Cents {
        Cents(self.0.saturating_add(rhs.0))
    }
}

impl Sub for Cents {
    type Output = Cents;
    fn sub(self, rhs: Cents) -> Cents {
        Cents(self.0.saturating_sub(rhs.0))
    }
}

impl Neg for Cents {
    type Output = Cents;
    fn neg(self) -> Cents {
        Cents(self.0.saturating_neg())
    }
}

impl AddAssign for Cents {
    fn add_assign(&mut self, rhs: Cents) {
        *self = *self + rhs;
    }
}

impl SubAssign for Cents {
    fn sub_assign(&mut self, rhs: Cents) {
        *self = *self - rhs;
    }
}

impl Sum for Cents {
    fn sum<I: Iterator<Item = Cents>>(iter: I) -> Cents {
        iter.fold(Cents::ZERO, Add::add)
    }
}

impl<'a> Sum<&'a Cents> for Cents {
    fn sum<I: Iterator<Item = &'a Cents>>(iter: I) -> Cents {
        iter.copied().sum()
    }
}
