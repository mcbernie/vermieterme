//! Tests for distribution keys, rounding edge cases, credits and prepayment suggestions.

use vermieterme_domain::billing::{
    CostPosition, Period, UnitBasis, UnitStatement, effective_key, suggest_next_prepayment,
    unit_statement,
};
use vermieterme_domain::{CalendarDate, CalendarMonths, Cents, DistributionKey};

use DistributionKey::{Mea, PerAttachment, PerNotice, PerUnit};

/// Unit with half of the shares of a two-unit property.
const HALF: UnitBasis = UnitBasis {
    shares: 1,
    total_shares: 2,
    unit_count: 2,
};

fn position(total: i64, key: DistributionKey) -> CostPosition {
    CostPosition {
        total_amount: Some(Cents::new(total)),
        unit_amount: None,
        enabled: true,
        key,
    }
}

/// Test helper: statement of `HALF` for a period given as ISO dates.
#[allow(clippy::unwrap_used)]
fn statement(costs: &[CostPosition], monthly: i64, start: &str, end: &str) -> UnitStatement {
    let period = Period {
        start: CalendarDate::parse(start).unwrap(),
        end: CalendarDate::parse(end).unwrap(),
    };
    unit_statement(costs, &HALF, Cents::new(monthly), period, &CalendarMonths).unwrap()
}

#[test]
fn key_codes_labels_and_serde() {
    for key in DistributionKey::ALL {
        assert_eq!(DistributionKey::from_code(key.code()), Ok(key));
        assert_eq!(DistributionKey::from_label(key.label()), Ok(key));
        assert_eq!(
            serde_json::to_string(&key).unwrap(),
            format!("\"{}\"", key.code())
        );
    }
    assert_eq!(
        DistributionKey::from_label("siehe Anlage"),
        Ok(PerAttachment)
    );
    assert_eq!(DistributionKey::from_label("laut Bescheid"), Ok(PerNotice));
    assert_eq!("per_unit".parse::<DistributionKey>(), Ok(PerUnit));
    assert!(DistributionKey::from_code("MEA").is_err());
    assert!(serde_json::from_str::<DistributionKey>("\"nach Anlage\"").is_err());
    assert_eq!(effective_key(None, Mea), Mea);
    assert_eq!(effective_key(Some(PerUnit), Mea), PerUnit);
}

#[test]
fn legacy_labels_ignore_case_and_surrounding_whitespace() {
    assert_eq!(DistributionKey::from_label("mea"), Ok(Mea));
    assert_eq!(
        DistributionKey::from_label("  Laut Bescheid "),
        Ok(PerNotice)
    );
    assert_eq!(
        DistributionKey::from_label("SIEHE ANLAGE"),
        Ok(PerAttachment)
    );
    assert_eq!(
        DistributionKey::from_label("\tnach einheiten\n"),
        Ok(PerUnit)
    );
    assert!(DistributionKey::from_label("lautBescheid").is_err());
    assert!(DistributionKey::from_label("nach Anlage").is_err());
    assert!(DistributionKey::from_label("").is_err());
}

#[test]
fn rounding_happens_per_position_before_summing() {
    // Three positions of 1 cent at 1/2: each rounds to 1 cent, the sum is 3 (exact: 1.5).
    let costs = [position(1, Mea), position(1, Mea), position(1, Mea)];
    let s = statement(&costs, 0, "2025-01-01", "2025-01-31");
    assert_eq!(s.unit_costs, Cents::new(3));
    assert_eq!(s.total_costs, Cents::new(3));
}

#[test]
fn credits_reduce_costs_and_can_lead_to_refunds() {
    let costs = [position(60_000, Mea), position(-10_001, Mea)];
    let s = statement(&costs, 2_000, "2024-07-01", "2025-06-30");
    // 30000 + (-5000.5 -> -5001)
    assert_eq!(s.unit_costs, Cents::new(24_999));
    assert_eq!(s.total_costs, Cents::new(49_999));
    assert_eq!(s.prepayment, Cents::new(24_000));
    assert_eq!(s.result(), Cents::new(999));
    assert_eq!(s.difference(), Cents::new(-999));
}

#[test]
fn suggestion_rounds_up_to_five_euro_steps() {
    let suggest = |monthly: i64, result: i64| {
        suggest_next_prepayment(Cents::new(monthly), Cents::new(result))
    };
    assert_eq!(suggest(10_000, 6_000), Some(Cents::new(10_500)));
    assert_eq!(suggest(10_000, 6_001), Some(Cents::new(11_000)));
    assert_eq!(suggest(10_000, 1), Some(Cents::new(10_500)));
    assert_eq!(suggest(8_550, 3_000), Some(Cents::new(9_000)));
    assert_eq!(suggest(0, 60), Some(Cents::new(500)));
    assert_eq!(suggest(9_500, 6_000), Some(Cents::new(10_000)));
}

#[test]
fn no_suggestion_for_refund_or_balanced_result() {
    assert_eq!(
        suggest_next_prepayment(Cents::new(10_000), Cents::ZERO),
        None
    );
    assert_eq!(
        suggest_next_prepayment(Cents::new(10_000), Cents::new(-1)),
        None
    );
}
