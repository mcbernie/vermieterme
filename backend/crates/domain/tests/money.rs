//! Tests for cent amounts: parsing, rounding, formatting, arithmetic.

use vermieterme_domain::Cents;
use vermieterme_domain::money::MoneyError;

#[test]
fn parses_euro_text() {
    assert_eq!(Cents::parse_euros("1234.56"), Ok(Cents::new(123_456)));
    assert_eq!(Cents::parse_euros("85.5"), Ok(Cents::new(8550)));
    assert_eq!(Cents::parse_euros("-0.05"), Ok(Cents::new(-5)));
    assert_eq!(Cents::parse_euros("0"), Ok(Cents::ZERO));
    assert_eq!(Cents::parse_euros("12,50"), Err(MoneyError::Invalid));
    assert_eq!(Cents::parse_euros(""), Err(MoneyError::Invalid));
}

#[test]
fn rounds_half_cent_away_from_zero() {
    assert_eq!(Cents::parse_euros("0.005"), Ok(Cents::new(1)));
    assert_eq!(Cents::parse_euros("-0.005"), Ok(Cents::new(-1)));
    assert_eq!(Cents::parse_euros("0.0049"), Ok(Cents::ZERO));
    assert_eq!(Cents::parse_euros("2.345"), Ok(Cents::new(235)));
    assert_eq!(Cents::parse_euros("-2.345"), Ok(Cents::new(-235)));
}

#[test]
fn converts_legacy_floats_by_their_decimal_text() {
    assert_eq!(Cents::from_euros_f64(0.1 + 0.2), Ok(Cents::new(30)));
    // (1.005 * 100).round() would give 100.
    assert_eq!(Cents::from_euros_f64(1.005), Ok(Cents::new(101)));
    assert_eq!(Cents::from_euros_f64(1234.567), Ok(Cents::new(123_457)));
    assert_eq!(Cents::from_euros_f64(-80.0), Ok(Cents::new(-8000)));
    assert_eq!(Cents::from_euros_f64(f64::NAN), Err(MoneyError::Invalid));
    assert_eq!(
        Cents::from_euros_f64(f64::INFINITY),
        Err(MoneyError::Invalid)
    );
    assert_eq!(Cents::from_euros_f64(1e13), Err(MoneyError::OutOfRange));
}

#[test]
fn legacy_float_extremes_give_meaningful_results() {
    assert_eq!(Cents::from_euros_f64(1e40), Err(MoneyError::OutOfRange));
    assert_eq!(Cents::from_euros_f64(-1e300), Err(MoneyError::OutOfRange));
    assert_eq!(Cents::from_euros_f64(f64::MAX), Err(MoneyError::OutOfRange));
    assert_eq!(Cents::from_euros_f64(1e-320), Ok(Cents::ZERO));
    assert_eq!(Cents::from_euros_f64(-1e-10), Ok(Cents::ZERO));
    assert_eq!(Cents::from_euros_f64(0.0049), Ok(Cents::ZERO));
    assert_eq!(
        Cents::parse_euros(&"9".repeat(40)),
        Err(MoneyError::OutOfRange)
    );
}

#[test]
fn limit_is_checked() {
    assert!(Cents::new(Cents::LIMIT).is_within_limit());
    assert!(Cents::new(-Cents::LIMIT).is_within_limit());
    assert!(!Cents::new(Cents::LIMIT + 1).is_within_limit());
    assert_eq!(
        Cents::parse_euros("10000000000.01"),
        Err(MoneyError::OutOfRange)
    );
}

#[test]
fn converts_to_euro_float_for_json() {
    assert_eq!(Cents::new(12_345).to_euros_f64(), 123.45);
    assert_eq!(Cents::new(-5).to_euros_f64(), -0.05);
}

#[test]
fn mul_ratio_rounds_half_away_from_zero() {
    assert_eq!(Cents::new(1).mul_ratio(1, 2), Some(Cents::new(1)));
    assert_eq!(Cents::new(-1).mul_ratio(1, 2), Some(Cents::new(-1)));
    assert_eq!(Cents::new(3).mul_ratio(1, 2), Some(Cents::new(2)));
    assert_eq!(Cents::new(-3).mul_ratio(1, 2), Some(Cents::new(-2)));
    assert_eq!(Cents::new(5).mul_ratio(1, 4), Some(Cents::new(1)));
    assert_eq!(Cents::new(10_000).mul_ratio(1, 3), Some(Cents::new(3333)));
    assert_eq!(Cents::new(20_000).mul_ratio(1, 3), Some(Cents::new(6667)));
    assert_eq!(Cents::new(100).mul_ratio(1, 0), None);
}

#[test]
fn arithmetic_and_sum() {
    let a = Cents::new(150);
    let b = Cents::new(-200);
    assert_eq!(a + b, Cents::new(-50));
    assert_eq!(a - b, Cents::new(350));
    assert_eq!(-a, Cents::new(-150));
    assert_eq!(a.times(12), Cents::new(1800));
    let mut c = a;
    c += b;
    c -= Cents::new(1);
    assert_eq!(c, Cents::new(-51));
    let total: Cents = [a, b, Cents::new(7)].iter().sum();
    assert_eq!(total, Cents::new(-43));
    assert_eq!(Cents::new(i64::MAX) + Cents::new(1), Cents::new(i64::MAX));
}

#[test]
fn formats_german_and_plain() {
    assert_eq!(Cents::ZERO.format_de(), "0,00");
    assert_eq!(Cents::new(5).format_de(), "0,05");
    assert_eq!(Cents::new(-5).format_de(), "-0,05");
    assert_eq!(Cents::new(100_000).format_de(), "1.000,00");
    assert_eq!(Cents::new(123_456_789).format_de(), "1.234.567,89");
    assert_eq!(Cents::new(-12_345_678).format_de(), "-123.456,78");
    assert_eq!(Cents::new(123_456).format_eur(), "1.234,56\u{a0}€");
    assert_eq!(Cents::new(-5).to_string(), "-0.05");
    assert_eq!(Cents::new(123_456).to_string(), "1234.56");
}

#[test]
fn serializes_as_plain_integer() {
    assert_eq!(
        serde_json::to_string(&Cents::new(12_345)).ok(),
        Some("12345".into())
    );
    let parsed: Cents = serde_json::from_str("-42").unwrap();
    assert_eq!(parsed, Cents::new(-42));
}
