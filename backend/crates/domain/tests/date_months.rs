//! Tests for calendar dates and the prepayment month rule.

use vermieterme_domain::date::DateError;
use vermieterme_domain::months::billing_months;
use vermieterme_domain::{CalendarDate, CalendarMonths, MonthRule};

/// Test helper: parses a date literal of the test.
#[allow(clippy::unwrap_used)]
fn d(text: &str) -> CalendarDate {
    CalendarDate::parse(text).unwrap()
}

#[test]
fn parses_and_prints_iso_dates() {
    let date = d("2025-01-31");
    assert_eq!((date.year(), date.month(), date.day()), (2025, 1, 31));
    assert_eq!(date.to_string(), "2025-01-31");
    assert_eq!(
        "0001-01-01".parse::<CalendarDate>().map(|d| d.to_string()),
        Ok("0001-01-01".into())
    );
    assert_eq!(date.to_app_timestamp(), "2025-01-31T00:00:00.000Z");
}

#[test]
fn rejects_malformed_or_impossible_dates() {
    for bad in [
        "",
        "2025-1-01",
        "2025-01-1",
        "25-01-01",
        "2025/01/01",
        "2025-01-01T00:00:00.000Z",
        "2025-13-01",
        "2025-00-10",
        "2025-04-31",
        "2023-02-29",
        "0000-01-01",
        "+202-01-01",
    ] {
        assert_eq!(CalendarDate::parse(bad), Err(DateError::Invalid), "{bad}");
    }
}

#[test]
fn leap_day_exists_only_in_leap_years() {
    assert!(CalendarDate::parse("2024-02-29").is_ok());
    assert!(CalendarDate::parse("2000-02-29").is_ok());
    assert!(CalendarDate::parse("1900-02-29").is_err());
}

#[test]
fn next_day_handles_month_year_and_leap_boundaries() {
    let next = |text: &str| d(text).next_day().map(|n| n.to_string());
    assert_eq!(next("2024-02-28"), Some("2024-02-29".into()));
    assert_eq!(next("2024-02-29"), Some("2024-03-01".into()));
    assert_eq!(next("2023-02-28"), Some("2023-03-01".into()));
    assert_eq!(next("2025-12-31"), Some("2026-01-01".into()));
    assert_eq!(next("9999-12-31"), None);
}

#[test]
fn serde_uses_iso_text() {
    let json = serde_json::to_string(&d("2025-03-09")).unwrap();
    assert_eq!(json, "\"2025-03-09\"");
    let back: CalendarDate = serde_json::from_str(&json).unwrap();
    assert_eq!(back, d("2025-03-09"));
    assert!(serde_json::from_str::<CalendarDate>("\"2025-02-30\"").is_err());
}

#[test]
fn full_year_has_twelve_months() {
    assert_eq!(billing_months(d("2025-01-01"), d("2025-12-31")), 12);
}

#[test]
fn same_month_counts_once() {
    assert_eq!(billing_months(d("2025-03-01"), d("2025-03-31")), 1);
    assert_eq!(billing_months(d("2025-03-10"), d("2025-03-10")), 1);
}

#[test]
fn every_touched_month_counts_fully() {
    assert_eq!(billing_months(d("2025-01-31"), d("2025-02-01")), 2);
    assert_eq!(billing_months(d("2025-01-15"), d("2025-03-01")), 3);
}

#[test]
fn year_change_is_counted_across() {
    assert_eq!(billing_months(d("2024-11-15"), d("2025-02-10")), 4);
    assert_eq!(billing_months(d("2024-07-01"), d("2025-06-30")), 12);
    assert_eq!(billing_months(d("2024-01-01"), d("2025-12-31")), 24);
}

#[test]
fn leap_february_is_one_month() {
    assert_eq!(billing_months(d("2024-02-01"), d("2024-02-29")), 1);
    assert_eq!(billing_months(d("2024-02-29"), d("2024-03-01")), 2);
}

#[test]
fn end_before_start_gives_one_month() {
    assert_eq!(billing_months(d("2025-06-01"), d("2025-01-01")), 1);
}

#[test]
fn rule_is_exchangeable() {
    /// Example of another rule: always bill twelve months.
    struct AlwaysTwelve;
    impl MonthRule for AlwaysTwelve {
        fn months(&self, _: CalendarDate, _: CalendarDate) -> u32 {
            12
        }
    }
    let rules: [&dyn MonthRule; 2] = [&CalendarMonths, &AlwaysTwelve];
    let months: Vec<u32> = rules
        .iter()
        .map(|r| r.months(d("2025-03-01"), d("2025-04-30")))
        .collect();
    assert_eq!(months, vec![2, 12]);
}

#[test]
fn from_date_keeps_the_year_range() {
    use time::{Date, Month};
    let date = |year| Date::from_calendar_date(year, Month::January, 1).unwrap();
    assert_eq!(CalendarDate::from_date(date(0)), Err(DateError::Invalid));
    assert_eq!(CalendarDate::from_date(date(-1)), Err(DateError::Invalid));
    assert_eq!(
        CalendarDate::from_date(date(1)).map(|d| d.to_string()),
        Ok("0001-01-01".into())
    );
    assert_eq!(
        CalendarDate::from_date(date(9999)).map(|d| d.year()),
        Ok(9999)
    );
    assert_eq!(CalendarDate::from_ymd(0, 1, 1), Err(DateError::Invalid));
    assert_eq!(
        CalendarDate::from_ymd(10_000, 1, 1),
        Err(DateError::Invalid)
    );
}
