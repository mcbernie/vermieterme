//! Tests for distribution keys, rounding edge cases, credits and prepayment suggestions.

use vermieterme_domain::billing::{
    CostPosition, Period, UnitBasis, effective_key, mea_share, per_unit_share,
    suggest_next_prepayment, unit_statement,
};
use vermieterme_domain::{CalendarDate, CalendarMonths, Cents, DistributionKey};

use DistributionKey::{Mea, PerAttachment, PerNotice, PerUnit};

fn basis(shares: i64, total_shares: i64, unit_count: i64) -> UnitBasis {
    UnitBasis {
        shares,
        total_shares,
        unit_count,
    }
}

fn position(total: i64, unit: Option<i64>, key: DistributionKey) -> CostPosition {
    CostPosition {
        total_amount: Some(Cents::new(total)),
        unit_amount: unit.map(Cents::new),
        enabled: true,
        key,
    }
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
fn mea_rounds_half_cent_away_from_zero() {
    assert_eq!(mea_share(Cents::new(1), 1, 2), Cents::new(1));
    assert_eq!(mea_share(Cents::new(3), 1, 2), Cents::new(2));
    assert_eq!(mea_share(Cents::new(-1), 1, 2), Cents::new(-1));
    assert_eq!(mea_share(Cents::new(-3), 1, 2), Cents::new(-2));
    assert_eq!(mea_share(Cents::new(10_000), 1, 3), Cents::new(3333));
    assert_eq!(mea_share(Cents::new(10_000), 0, 1000), Cents::ZERO);
}

#[test]
fn property_without_shares_or_units_gives_zero() {
    assert_eq!(mea_share(Cents::new(10_000), 100, 0), Cents::ZERO);
    assert_eq!(per_unit_share(Cents::new(10_000), 0), Cents::ZERO);
}

#[test]
fn per_unit_splits_equally_and_rounds() {
    assert_eq!(per_unit_share(Cents::new(1000), 3), Cents::new(333));
    assert_eq!(per_unit_share(Cents::new(2000), 3), Cents::new(667));
    assert_eq!(per_unit_share(Cents::new(1001), 2), Cents::new(501));
    assert_eq!(per_unit_share(Cents::new(-1001), 2), Cents::new(-501));
    let p = position(120_000, None, PerUnit);
    assert_eq!(p.unit_share(&basis(1, 1000, 10)), Some(Cents::new(12_000)));
}

#[test]
fn stored_unit_amount_wins_for_every_key() {
    for key in DistributionKey::ALL {
        let p = position(100_000, Some(4_242), key);
        assert_eq!(
            p.unit_share(&basis(500, 1000, 2)),
            Some(Cents::new(4_242)),
            "{key}"
        );
    }
}

#[test]
fn entered_keys_without_unit_amount_give_zero() {
    for key in [PerNotice, PerAttachment] {
        assert_eq!(
            position(100_000, None, key).unit_share(&basis(500, 1000, 2)),
            Some(Cents::ZERO)
        );
    }
}

#[test]
fn disabled_or_empty_positions_count_nowhere() {
    let mut disabled = position(100_000, Some(5_000), PerNotice);
    disabled.enabled = false;
    assert!(!disabled.counts());
    assert_eq!(disabled.unit_share(&basis(1, 2, 2)), None);
    let empty = CostPosition {
        total_amount: None,
        ..position(0, Some(5_000), PerNotice)
    };
    assert!(!empty.counts());
    assert_eq!(empty.unit_share(&basis(1, 2, 2)), None);
}

#[test]
fn rounding_happens_per_position_before_summing() {
    // Three positions of 1 cent at 1/2: each rounds to 1 cent, the sum is 3 (exact: 1.5).
    let costs = [
        position(1, None, Mea),
        position(1, None, Mea),
        position(1, None, Mea),
    ];
    let period = Period {
        start: CalendarDate::parse("2025-01-01").unwrap(),
        end: CalendarDate::parse("2025-01-31").unwrap(),
    };
    let s = unit_statement(
        &costs,
        &basis(1, 2, 2),
        Cents::ZERO,
        period,
        &CalendarMonths,
    );
    assert_eq!(s.unit_costs, Cents::new(3));
    assert_eq!(s.total_costs, Cents::new(3));
}

#[test]
fn credits_reduce_costs_and_can_lead_to_refunds() {
    let costs = [position(60_000, None, Mea), position(-10_001, None, Mea)];
    let period = Period {
        start: CalendarDate::parse("2024-07-01").unwrap(),
        end: CalendarDate::parse("2025-06-30").unwrap(),
    };
    let s = unit_statement(
        &costs,
        &basis(1, 2, 2),
        Cents::new(2_000),
        period,
        &CalendarMonths,
    );
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
