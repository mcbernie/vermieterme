//! Exact integer division with explicit rounding modes.
//!
//! All money and index arithmetic ends here, so the rounding rule is defined once.
//! Intermediate values use `i128` so products of cent amounts never overflow.

/// Divides `num / den` and rounds half away from zero (kaufmännische Rundung).
///
/// Returns `None` when `den` is zero.
pub(crate) fn div_round_half_away(num: i128, den: i128) -> Option<i128> {
    if den == 0 {
        return None;
    }
    // Normalise to a positive divisor so only the numerator carries the sign.
    let (num, den) = if den < 0 { (-num, -den) } else { (num, den) };
    let quotient = num / den;
    let remainder = num % den;
    // `remainder` has the sign of `num`; a remainder of at least half rounds outwards.
    if remainder.abs() * 2 >= den {
        Some(quotient + num.signum())
    } else {
        Some(quotient)
    }
}

/// Divides `num / den` and rounds towards positive infinity.
///
/// Returns `None` when `den` is zero.
pub(crate) fn div_ceil(num: i128, den: i128) -> Option<i128> {
    if den == 0 {
        return None;
    }
    let (num, den) = if den < 0 { (-num, -den) } else { (num, den) };
    let quotient = num / den;
    // Truncation already rounds negative quotients up; only positive remainders need a step.
    if num % den > 0 {
        Some(quotient + 1)
    } else {
        Some(quotient)
    }
}

/// Converts an `i128` to `i64`, saturating at the bounds.
///
/// Only reached with inputs outside [`crate::Cents::LIMIT`]; saturating keeps the
/// functions total without panics.
pub(crate) fn saturate_i64(value: i128) -> i64 {
    i64::try_from(value).unwrap_or(if value < 0 { i64::MIN } else { i64::MAX })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_rounds_away_from_zero() {
        assert_eq!(div_round_half_away(5, 10), Some(1));
        assert_eq!(div_round_half_away(-5, 10), Some(-1));
        assert_eq!(div_round_half_away(15, 10), Some(2));
        assert_eq!(div_round_half_away(-15, 10), Some(-2));
        assert_eq!(div_round_half_away(25, -10), Some(-3));
    }

    #[test]
    fn below_half_rounds_towards_zero() {
        assert_eq!(div_round_half_away(4, 10), Some(0));
        assert_eq!(div_round_half_away(-4, 10), Some(0));
        assert_eq!(div_round_half_away(149, 10), Some(15));
        assert_eq!(div_round_half_away(144, 10), Some(14));
        assert_eq!(div_round_half_away(1, 3), Some(0));
        assert_eq!(div_round_half_away(2, 3), Some(1));
    }

    #[test]
    fn zero_divisor_is_none() {
        assert_eq!(div_round_half_away(1, 0), None);
        assert_eq!(div_ceil(1, 0), None);
    }

    #[test]
    fn ceil_division() {
        assert_eq!(div_ceil(10, 5), Some(2));
        assert_eq!(div_ceil(11, 5), Some(3));
        assert_eq!(div_ceil(-11, 5), Some(-2));
        assert_eq!(div_ceil(11, -5), Some(-2));
        assert_eq!(div_ceil(0, 5), Some(0));
    }

    #[test]
    fn saturation() {
        assert_eq!(saturate_i64(i128::from(i64::MAX) + 1), i64::MAX);
        assert_eq!(saturate_i64(i128::from(i64::MIN) - 1), i64::MIN);
        assert_eq!(saturate_i64(42), 42);
    }
}
