//! Tests for the unit share of a cost position: MEA, per unit, entered amounts and
//! inconsistent stored bases (error only where the basis is really needed).

use vermieterme_domain::billing::{
    CostPosition, Period, ShareError, UnitBasis, UnitStatement, unit_statement,
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

/// Test helper: statement over the year 2025 without prepayment.
#[allow(clippy::unwrap_used)]
fn statement(costs: &[CostPosition], basis: &UnitBasis) -> Result<UnitStatement, ShareError> {
    let period = Period {
        start: CalendarDate::parse("2025-01-01").unwrap(),
        end: CalendarDate::parse("2025-12-31").unwrap(),
    };
    unit_statement(costs, basis, Cents::ZERO, period, &CalendarMonths)
}

#[test]
fn mea_rounds_half_cent_away_from_zero() {
    let half = basis(1, 2, 2);
    assert_eq!(half.mea_share(Cents::new(1)), Ok(Cents::new(1)));
    assert_eq!(half.mea_share(Cents::new(3)), Ok(Cents::new(2)));
    assert_eq!(half.mea_share(Cents::new(-1)), Ok(Cents::new(-1)));
    assert_eq!(half.mea_share(Cents::new(-3)), Ok(Cents::new(-2)));
    assert_eq!(
        basis(1, 3, 3).mea_share(Cents::new(10_000)),
        Ok(Cents::new(3333))
    );
    assert_eq!(
        basis(0, 1000, 2).mea_share(Cents::new(10_000)),
        Ok(Cents::ZERO)
    );
}

#[test]
fn property_without_shares_gives_zero_only_for_units_without_shares() {
    assert_eq!(
        basis(0, 0, 1).mea_share(Cents::new(10_000)),
        Ok(Cents::ZERO)
    );
    assert_eq!(
        basis(100, 0, 1).mea_share(Cents::new(10_000)),
        Err(ShareError::InvalidShares)
    );
}

#[test]
fn mea_rejects_negative_or_excess_shares() {
    for b in [basis(-1, 1000, 2), basis(1001, 1000, 2), basis(5, 3, 2)] {
        assert_eq!(b.mea_share(Cents::new(100)), Err(ShareError::InvalidShares));
    }
}

#[test]
fn basis_can_be_checked_without_computing() {
    assert_eq!(
        basis(1001, 1000, 2).check_shares(),
        Err(ShareError::InvalidShares)
    );
    assert_eq!(
        basis(-1, 1000, 2).check_shares(),
        Err(ShareError::InvalidShares)
    );
    assert_eq!(
        basis(100, 0, 1).check_shares(),
        Err(ShareError::InvalidShares)
    );
    assert_eq!(basis(0, 0, 1).check_shares(), Ok(()));
    assert_eq!(basis(1000, 1000, 2).check_shares(), Ok(()));
    assert_eq!(basis(0, 0, 0).check_unit_count(), Err(ShareError::NoUnits));
    assert_eq!(basis(0, 0, -1).check_unit_count(), Err(ShareError::NoUnits));
    assert_eq!(basis(9, 1, 1).check_unit_count(), Ok(()));
}

#[test]
fn per_unit_splits_equally_and_needs_a_unit() {
    let three = basis(0, 0, 3);
    assert_eq!(three.per_unit_share(Cents::new(1000)), Ok(Cents::new(333)));
    assert_eq!(three.per_unit_share(Cents::new(2000)), Ok(Cents::new(667)));
    assert_eq!(
        basis(0, 0, 2).per_unit_share(Cents::new(1001)),
        Ok(Cents::new(501))
    );
    assert_eq!(
        basis(0, 0, 2).per_unit_share(Cents::new(-1001)),
        Ok(Cents::new(-501))
    );
    // per_unit ignores shares entirely, even inconsistent ones.
    assert_eq!(
        basis(9, 1, 10).per_unit_share(Cents::new(120_000)),
        Ok(Cents::new(12_000))
    );
    for count in [0, -3] {
        assert_eq!(
            basis(0, 0, count).per_unit_share(Cents::new(1)),
            Err(ShareError::NoUnits)
        );
    }
}

#[test]
fn stored_unit_amount_counts_for_entered_keys() {
    for key in [PerNotice, PerAttachment] {
        let p = position(100_000, Some(4_242), key);
        assert_eq!(
            p.unit_share(&basis(500, 1000, 2)),
            Ok(Some(Cents::new(4_242))),
            "{key}"
        );
        let empty = position(100_000, None, key);
        assert_eq!(
            empty.unit_share(&basis(500, 1000, 2)),
            Ok(Some(Cents::ZERO)),
            "{key}"
        );
    }
}

#[test]
fn stored_unit_amount_is_ignored_for_computed_keys() {
    // A stale amount from before a key change must not override the computed share (#81).
    for key in [Mea, PerUnit] {
        let p = position(100_000, Some(4_242), key);
        assert_eq!(
            p.unit_share(&basis(500, 1000, 2)),
            Ok(Some(Cents::new(50_000))),
            "{key}"
        );
    }
}

#[test]
fn disabled_or_empty_positions_count_nowhere() {
    let mut disabled = position(100_000, Some(5_000), Mea);
    disabled.enabled = false;
    assert!(!disabled.counts());
    // Not counting means the (invalid) basis is never looked at.
    assert_eq!(disabled.unit_share(&basis(9, 1, 0)), Ok(None));
    let empty = CostPosition {
        total_amount: None,
        ..position(0, Some(5_000), Mea)
    };
    assert!(!empty.counts());
    assert_eq!(empty.unit_share(&basis(9, 1, 0)), Ok(None));
}

#[test]
fn invalid_shares_do_not_block_a_bill_without_mea() {
    let broken = basis(1200, 1000, 2);
    let costs = [
        position(50_000, Some(21_030), PerNotice),
        position(9_999, Some(1_200), PerAttachment),
        position(10_000, None, PerUnit),
    ];
    let s = statement(&costs, &broken).map(|s| s.unit_costs);
    assert_eq!(s, Ok(Cents::new(21_030 + 1_200 + 5_000)));
}

#[test]
fn invalid_shares_fail_once_a_mea_position_counts() {
    let broken = basis(1200, 1000, 2);
    let costs = [
        position(50_000, Some(21_030), PerNotice),
        position(10_000, None, Mea),
    ];
    let err = statement(&costs, &broken).err();
    assert_eq!(err, Some(ShareError::InvalidShares));
    assert!(
        err.map(|e| e.to_string())
            .is_some_and(|m| m.contains("Miteigentumsanteile"))
    );
    // The same MEA position disabled: the bill computes again.
    let mut disabled = costs;
    disabled[1].enabled = false;
    assert!(statement(&disabled, &broken).is_ok());
}

#[test]
fn per_unit_position_fails_without_units() {
    let costs = [position(10_000, None, PerUnit)];
    assert_eq!(
        statement(&costs, &basis(0, 0, 0)).err(),
        Some(ShareError::NoUnits)
    );
}

#[test]
fn entered_unit_amount_counts_without_total_for_entered_keys() {
    // Decision 39: e.g. property tax per flat, no property total entered (#80).
    for key in [PerNotice, PerAttachment] {
        let p = CostPosition {
            total_amount: None,
            ..position(0, Some(21_030), key)
        };
        assert!(p.counts(), "{key}");
        assert_eq!(
            p.unit_share(&basis(9, 1, 0)),
            Ok(Some(Cents::new(21_030))),
            "{key}"
        );
        let s = statement(&[p], &basis(9, 1, 0));
        assert_eq!(
            s.map(|s| (s.unit_costs, s.total_costs)),
            Ok((Cents::new(21_030), Cents::ZERO))
        );
    }
}

#[test]
fn entered_keys_without_any_amount_do_not_count() {
    for key in [PerNotice, PerAttachment] {
        let p = CostPosition {
            total_amount: None,
            ..position(0, None, key)
        };
        assert!(!p.counts(), "{key}");
        assert_eq!(p.unit_share(&basis(1, 2, 2)), Ok(None), "{key}");
        let mut disabled = position(0, Some(5_000), key);
        disabled.total_amount = None;
        disabled.enabled = false;
        assert!(!disabled.counts(), "{key}");
    }
}

#[test]
fn computed_keys_need_the_total_even_with_a_unit_amount() {
    for key in [Mea, PerUnit] {
        let p = CostPosition {
            total_amount: None,
            ..position(0, Some(5_000), key)
        };
        assert!(!p.counts(), "{key}");
        // Not counting: the broken basis is never looked at.
        assert_eq!(
            statement(&[p], &basis(9, 1, 0)).map(|s| s.unit_costs),
            Ok(Cents::ZERO)
        );
    }
}
