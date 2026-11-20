//! Exact parsing of decimal strings into scaled integers.
//!
//! Used for euro amounts (scale 2) and VPI values (scale 1). Parsing the decimal
//! text instead of multiplying a float avoids errors such as `1.005 * 100 = 100.4999…`.

/// Most significant integer digits accepted; together with the kept fraction digits
/// this stays far below the 38 digits of `i128`.
const MAX_INT_DIGITS: usize = 30;

/// Why a decimal text could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DecimalError {
    /// Not of the form `[+-]digits[.digits]`.
    Malformed,
    /// More than [`MAX_INT_DIGITS`] integer digits.
    TooLarge,
}

/// Parses `text` (`[+-]digits[.digits]`, `.` as separator, no exponent) and returns
/// the value multiplied by `10^scale`, rounded half away from zero.
///
/// Any number of fraction digits is accepted (tiny values round to 0); integer parts
/// longer than [`MAX_INT_DIGITS`] digits give [`DecimalError::TooLarge`].
pub(crate) fn parse_scaled(text: &str, scale: u32) -> Result<i128, DecimalError> {
    let text = text.trim();
    let (negative, unsigned) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let (int_part, frac_part) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let all_digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if (int_part.is_empty() && frac_part.is_empty())
        || !all_digits(int_part)
        || !all_digits(frac_part)
    {
        return Err(DecimalError::Malformed);
    }
    let int_part = int_part.trim_start_matches('0');
    if int_part.len() > MAX_INT_DIGITS {
        return Err(DecimalError::TooLarge);
    }
    // Half away from zero only depends on the first dropped digit (>= 5 rounds up),
    // so the fraction is cut to `scale` digits plus that one.
    let scale = usize::try_from(scale).map_err(|_| DecimalError::TooLarge)?;
    let frac = frac_part.as_bytes();
    let kept = &frac[..frac.len().min(scale)];
    let round_up = frac.get(scale).is_some_and(|d| *d >= b'5');
    let padding = std::iter::repeat_n(b'0', scale - kept.len());
    let digits = int_part.bytes().chain(kept.iter().copied()).chain(padding);
    let value = digits.fold(0i128, |acc, d| acc * 10 + i128::from(d - b'0')) + i128::from(round_up);
    Ok(if negative { -value } else { value })
}

#[cfg(test)]
mod tests {
    use super::{DecimalError, parse_scaled};

    #[test]
    fn parses_plain_values() {
        assert_eq!(parse_scaled("12", 2), Ok(1200));
        assert_eq!(parse_scaled("12.5", 2), Ok(1250));
        assert_eq!(parse_scaled("12.34", 2), Ok(1234));
        assert_eq!(parse_scaled(".5", 2), Ok(50));
        assert_eq!(parse_scaled("5.", 2), Ok(500));
        assert_eq!(parse_scaled(" 117.6 ", 1), Ok(1176));
        assert_eq!(parse_scaled("+3", 1), Ok(30));
        assert_eq!(parse_scaled("007.5", 1), Ok(75));
    }

    #[test]
    fn rounds_extra_digits_half_away_from_zero() {
        assert_eq!(parse_scaled("1.005", 2), Ok(101));
        assert_eq!(parse_scaled("1.004", 2), Ok(100));
        assert_eq!(parse_scaled("-1.005", 2), Ok(-101));
        assert_eq!(parse_scaled("119.35", 1), Ok(1194));
        assert_eq!(parse_scaled("119.349999", 1), Ok(1193));
        assert_eq!(parse_scaled("0.995", 2), Ok(100));
    }

    #[test]
    fn tiny_values_round_to_zero() {
        let tiny = format!("0.{}1", "0".repeat(400));
        assert_eq!(parse_scaled(&tiny, 2), Ok(0));
        assert_eq!(parse_scaled(&format!("-{tiny}"), 2), Ok(0));
    }

    #[test]
    fn rejects_malformed_input() {
        for bad in [
            "", "-", ".", "1,5", "1e3", "abc", "1.2.3", "--1", "12abc", "NaN",
        ] {
            assert_eq!(parse_scaled(bad, 2), Err(DecimalError::Malformed), "{bad}");
        }
    }

    #[test]
    fn rejects_huge_values() {
        assert_eq!(
            parse_scaled(&"9".repeat(31), 2),
            Err(DecimalError::TooLarge)
        );
        assert_eq!(parse_scaled(&"9".repeat(30), 2).map(|v| v > 0), Ok(true));
        assert_eq!(parse_scaled(&format!("{}1", "0".repeat(60)), 2), Ok(100));
    }
}
