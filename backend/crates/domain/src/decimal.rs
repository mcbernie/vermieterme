//! Exact parsing of decimal strings into scaled integers.
//!
//! Used for euro amounts (scale 2) and VPI values (scale 1). Parsing the decimal
//! text instead of multiplying a float avoids errors such as `1.005 * 100 = 100.4999…`.

use crate::rounding::div_round_half_away;

/// Parses `text` (`[+-]digits[.digits]`, `.` as separator, no exponent) and returns
/// the value multiplied by `10^scale`, rounded half away from zero.
///
/// Returns `None` for malformed input or values that do not fit into `i128`.
pub(crate) fn parse_scaled(text: &str, scale: u32) -> Option<i128> {
    let text = text.trim();
    let (negative, unsigned) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let (int_part, frac_part) = match unsigned.split_once('.') {
        Some((int_part, frac_part)) => (int_part, frac_part),
        None => (unsigned, ""),
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return None;
    }
    let all_digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if !all_digits(int_part) || !all_digits(frac_part) {
        return None;
    }
    // value = digits / 10^frac_len; scaled = digits * 10^scale / 10^frac_len.
    let digits = accumulate(int_part.bytes().chain(frac_part.bytes()))?;
    let frac_len = u32::try_from(frac_part.len()).ok()?;
    let scaled = if frac_len <= scale {
        digits.checked_mul(10i128.checked_pow(scale - frac_len)?)?
    } else {
        div_round_half_away(digits, 10i128.checked_pow(frac_len - scale)?)?
    };
    Some(if negative { -scaled } else { scaled })
}

/// Folds ASCII digits into an integer, `None` on overflow.
fn accumulate(digits: impl Iterator<Item = u8>) -> Option<i128> {
    digits.into_iter().try_fold(0i128, |acc, digit| {
        acc.checked_mul(10)?.checked_add(i128::from(digit - b'0'))
    })
}

#[cfg(test)]
mod tests {
    use super::parse_scaled;

    #[test]
    fn parses_plain_values() {
        assert_eq!(parse_scaled("12", 2), Some(1200));
        assert_eq!(parse_scaled("12.5", 2), Some(1250));
        assert_eq!(parse_scaled("12.34", 2), Some(1234));
        assert_eq!(parse_scaled(".5", 2), Some(50));
        assert_eq!(parse_scaled("5.", 2), Some(500));
        assert_eq!(parse_scaled(" 117.6 ", 1), Some(1176));
        assert_eq!(parse_scaled("+3", 1), Some(30));
    }

    #[test]
    fn rounds_extra_digits_half_away_from_zero() {
        assert_eq!(parse_scaled("1.005", 2), Some(101));
        assert_eq!(parse_scaled("1.004", 2), Some(100));
        assert_eq!(parse_scaled("-1.005", 2), Some(-101));
        assert_eq!(parse_scaled("119.35", 1), Some(1194));
        assert_eq!(parse_scaled("119.349999", 1), Some(1193));
    }

    #[test]
    fn rejects_malformed_input() {
        for bad in [
            "", "-", ".", "1,5", "1e3", "abc", "1.2.3", "--1", "12abc", "NaN",
        ] {
            assert_eq!(parse_scaled(bad, 2), None, "{bad}");
        }
    }

    #[test]
    fn rejects_overflow() {
        assert_eq!(parse_scaled(&"9".repeat(60), 2), None);
    }
}
