//! Port of `src/__tests__/lib/vpi.test.ts` plus the rent adjustment proposal.

use vermieterme_domain::vpi::{
    VpiDataPoint, VpiEntry, base_year_code, index_adjustment, latest_for_base_year, months_since,
    parse_bundesbank_response,
};
use vermieterme_domain::{CalendarDate, Cents, VpiValue};

/// Builds an SDMX-JSON body with the given period ids and observation values.
fn response(periods: &[&str], observations: &[(&str, &str)]) -> String {
    let values: Vec<String> = periods
        .iter()
        .map(|p| format!("{{\"id\":\"{p}\"}}"))
        .collect();
    let obs: Vec<String> = observations
        .iter()
        .map(|(i, v)| format!("\"{i}\":[{v}]"))
        .collect();
    format!(
        r#"{{"data":{{"structure":{{"dimensions":{{"observation":[{{"values":[{}]}}]}}}},
        "dataSets":[{{"series":{{"0:0:0:0:0:0:0:0":{{"observations":{{{}}}}}}}}}]}}}}"#,
        values.join(","),
        obs.join(",")
    )
}

fn point(year: i32, month: u8, tenths: i64) -> VpiDataPoint {
    VpiDataPoint {
        year,
        month,
        value: VpiValue::from_tenths(tenths),
    }
}

/// Test helper: parses a date literal of the test.
#[allow(clippy::unwrap_used)]
fn d(text: &str) -> CalendarDate {
    CalendarDate::parse(text).unwrap()
}

#[test]
fn has_codes_for_supported_base_years() {
    assert_eq!(base_year_code(2020), Some("I20"));
    assert_eq!(base_year_code(2015), Some("I15"));
    assert_eq!(base_year_code(2010), Some("I10"));
    assert_eq!(base_year_code(2000), None);
}

#[test]
fn parses_a_valid_response() {
    let body = response(
        &["2024-01", "2024-02", "2024-03"],
        &[("0", "\"117.6\""), ("1", "\"118.1\""), ("2", "\"118.6\"")],
    );
    let result = parse_bundesbank_response(&body).unwrap();
    assert_eq!(
        result,
        vec![
            point(2024, 1, 1176),
            point(2024, 2, 1181),
            point(2024, 3, 1186)
        ]
    );
}

#[test]
fn sorts_by_year_and_month() {
    let body = response(
        &["2025-01", "2024-12", "2024-11"],
        &[("0", "\"120.3\""), ("1", "\"120.5\""), ("2", "\"119.9\"")],
    );
    let result = parse_bundesbank_response(&body).unwrap();
    assert_eq!(
        result,
        vec![
            point(2024, 11, 1199),
            point(2024, 12, 1205),
            point(2025, 1, 1203)
        ]
    );
}

#[test]
fn rounds_values_to_one_decimal() {
    let body = response(&["2024-06"], &[("0", "\"119.35\"")]);
    assert_eq!(
        parse_bundesbank_response(&body).unwrap()[0].value,
        VpiValue::from_tenths(1194)
    );
}

#[test]
fn returns_empty_list_for_missing_data() {
    let body = r#"{"data":{"structure":{"dimensions":{"observation":[]}},"dataSets":[]}}"#;
    assert_eq!(parse_bundesbank_response(body).unwrap(), vec![]);
}

#[test]
fn skips_entries_with_invalid_period_format() {
    let body = response(
        &["2024-01", "invalid", "2024-03"],
        &[("0", "\"117.6\""), ("1", "\"999.9\""), ("2", "\"118.6\"")],
    );
    let months: Vec<u8> = parse_bundesbank_response(&body)
        .unwrap()
        .iter()
        .map(|p| p.month)
        .collect();
    assert_eq!(months, vec![1, 3]);
}

#[test]
fn skips_values_that_are_not_numbers() {
    let body = response(&["2024-01"], &[("0", "\"not-a-number\"")]);
    assert_eq!(parse_bundesbank_response(&body).unwrap(), vec![]);
}

#[test]
fn accepts_json_numbers_and_skips_impossible_months_and_indices() {
    let body = response(
        &["2024-01", "2024-13", "2024-02"],
        &[
            ("0", "117.6"),
            ("1", "\"118.0\""),
            ("2", "null"),
            ("7", "\"1.0\""),
            ("x", "\"1.0\""),
        ],
    );
    assert_eq!(
        parse_bundesbank_response(&body).unwrap(),
        vec![point(2024, 1, 1176)]
    );
}

#[test]
fn skips_single_malformed_observations() {
    let body = r#"{"data":{"structure":{"dimensions":{"observation":[{"values":
        [{"id":"2024-01"},{"id":"2024-02"},{"nope":1},{"id":"2024-04"}]}]}},
        "dataSets":[{"series":{
            "a":{"observations":{"0":["117.6"],"1":null,"2":["1.0"],"3":"118.0"}},
            "b":null,
            "c":{"observations":null}}}]}}"#;
    assert_eq!(
        parse_bundesbank_response(body).unwrap(),
        vec![point(2024, 1, 1176)]
    );
}

#[test]
fn tolerates_missing_or_null_series() {
    let body = r#"{"data":{"structure":{"dimensions":{"observation":[{"values":[{"id":"2024-01"}]}]}},
        "dataSets":[{"series":null}]}}"#;
    assert_eq!(parse_bundesbank_response(body).unwrap(), vec![]);
}

#[test]
fn treats_null_lists_like_missing_ones() {
    for body in [
        r#"{"data":{"structure":{"dimensions":{"observation":[{"values":[{"id":"2024-01"}]}]}},"dataSets":null}}"#,
        r#"{"data":{"structure":{"dimensions":{"observation":null}},"dataSets":[]}}"#,
        r#"{"data":{"structure":{"dimensions":{"observation":[{"values":null}]}},
            "dataSets":[{"series":{"a":{"observations":{"0":["117.6"]}}}}]}}"#,
        r#"{"data":{"structure":{"dimensions":{}}}}"#,
    ] {
        assert_eq!(parse_bundesbank_response(body).unwrap(), vec![], "{body}");
    }
}

#[test]
fn rejects_bodies_of_the_wrong_shape() {
    assert!(parse_bundesbank_response("not json").is_err());
    assert!(parse_bundesbank_response("{}").is_err());
}

#[test]
fn parses_values_from_text_and_float() {
    assert_eq!(VpiValue::parse("119.6"), Some(VpiValue::from_tenths(1196)));
    assert_eq!(
        VpiValue::from_f64(119.35),
        Some(VpiValue::from_tenths(1194))
    );
    assert_eq!(VpiValue::from_f64(f64::NAN), None);
}

#[test]
fn picks_the_latest_value_of_the_base_year() {
    let entry = |year, month, tenths, base_year| VpiEntry {
        year,
        month,
        value: VpiValue::from_tenths(tenths),
        base_year,
    };
    let entries = [
        entry(2024, 12, 1200, 2020),
        entry(2025, 2, 1210, 2020),
        entry(2025, 1, 1205, 2020),
        entry(2025, 3, 1300, 2015),
    ];
    assert_eq!(
        latest_for_base_year(&entries, 2020).map(|e| e.month),
        Some(2)
    );
    assert_eq!(latest_for_base_year(&entries, 2010), None);
}

#[test]
fn proposes_an_adjustment() {
    let a = index_adjustment(
        Cents::new(100_000),
        VpiValue::from_tenths(1100),
        VpiValue::from_tenths(1155),
        d("2023-03-15"),
        d("2025-03-01"),
        12,
    )
    .unwrap();
    assert_eq!(a.percentage_change_basis_points, 500);
    assert_eq!(a.suggested_rent, Cents::new(105_000));
    assert_eq!(a.months_since_last_adjustment, 24);
    assert!(a.eligible);
}

#[test]
fn rounds_change_and_rent_half_away_from_zero() {
    // (118.1 - 117.6) / 117.6 = 0.42517 % -> 43 bp; 850.50 * 118.1 / 117.6 = 854.1161 -> 854.12
    let a = index_adjustment(
        Cents::new(85_050),
        VpiValue::from_tenths(1176),
        VpiValue::from_tenths(1181),
        d("2024-06-01"),
        d("2025-05-31"),
        12,
    )
    .unwrap();
    assert_eq!(a.percentage_change_basis_points, 43);
    assert_eq!(a.suggested_rent, Cents::new(85_412));
    assert_eq!(a.months_since_last_adjustment, 11);
    assert!(!a.eligible);
    // Falling index with an exact half basis point: -2.5 bp -> -3 (old code: -2).
    let b = index_adjustment(
        Cents::new(1),
        VpiValue::from_tenths(4000),
        VpiValue::from_tenths(3999),
        d("2024-01-01"),
        d("2025-01-01"),
        12,
    )
    .unwrap();
    assert_eq!(b.percentage_change_basis_points, -3);
    assert_eq!(b.suggested_rent, Cents::new(1));
}

#[test]
fn no_proposal_without_rent_or_reference() {
    let ref_date = d("2024-01-01");
    let today = d("2025-01-01");
    assert_eq!(
        index_adjustment(
            Cents::ZERO,
            VpiValue::from_tenths(1100),
            VpiValue::from_tenths(1200),
            ref_date,
            today,
            12
        ),
        None
    );
    assert_eq!(
        index_adjustment(
            Cents::new(1),
            VpiValue::from_tenths(0),
            VpiValue::from_tenths(1200),
            ref_date,
            today,
            12
        ),
        None
    );
}

#[test]
fn months_since_ignores_the_day_and_may_be_negative() {
    assert_eq!(months_since(d("2024-01-31"), d("2024-02-01")), 1);
    assert_eq!(months_since(d("2024-12-15"), d("2025-01-14")), 1);
    assert_eq!(months_since(d("2025-03-01"), d("2025-01-01")), -2);
}
