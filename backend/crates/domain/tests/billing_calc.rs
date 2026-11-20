//! Port of `src/__tests__/features/billing-calc.test.ts` with euro amounts in cents.
//!
//! Fixture: property with 1000 shares, unit 1 (333 shares) and unit 2 (667 shares),
//! period 2025-01-01..2025-12-31. The TypeScript fixture's 1234.567 € has no exact
//! cent value and is 123457 cents here.

use vermieterme_domain::billing::{
    CostPosition, CostReview, Period, ReviewProgress, UnitBasis, UnitStatement, effective_key,
    mea_share, suggest_next_prepayment, unit_statement,
};
use vermieterme_domain::{CalendarDate, CalendarMonths, Cents, DistributionKey};

use DistributionKey::{Mea, PerAttachment, PerNotice};

const UNIT_1: UnitBasis = UnitBasis {
    shares: 333,
    total_shares: 1000,
    unit_count: 2,
};
const UNIT_2: UnitBasis = UnitBasis {
    shares: 667,
    total_shares: 1000,
    unit_count: 2,
};

fn position(
    total: Option<i64>,
    unit: Option<i64>,
    enabled: bool,
    key: DistributionKey,
) -> CostPosition {
    CostPosition {
        total_amount: total.map(Cents::new),
        unit_amount: unit.map(Cents::new),
        enabled,
        key,
    }
}

/// c1 Grundsteuer (MEA), c2 Wasser (laut Bescheid), c3 Müll (siehe Anlage),
/// c4 Strom (MEA, overridden to laut Bescheid), c5 Leer (MEA, disabled).
fn costs() -> Vec<CostPosition> {
    vec![
        position(Some(123_457), None, true, Mea),
        position(Some(50_000), Some(21_030), true, PerNotice),
        position(Some(0), Some(0), true, PerAttachment),
        position(
            Some(9_999),
            Some(1_200),
            true,
            effective_key(Some(PerNotice), Mea),
        ),
        position(Some(8_000), None, false, Mea),
    ]
}

/// Test helper: the calendar year 2025.
#[allow(clippy::unwrap_used)]
fn period() -> Period {
    Period {
        start: CalendarDate::parse("2025-01-01").unwrap(),
        end: CalendarDate::parse("2025-12-31").unwrap(),
    }
}

fn statement(costs: &[CostPosition], basis: &UnitBasis, monthly: i64) -> UnitStatement {
    unit_statement(costs, basis, Cents::new(monthly), period(), &CalendarMonths)
}

#[test]
fn computes_unit_costs_from_positions() {
    // MEA rounded per position: 123457 * 333/1000 = 41111.181 -> 41111,
    // 123457 * 667/1000 = 82345.819 -> 82346; other keys take the entered unit amount.
    assert_eq!(
        statement(&costs(), &UNIT_1, 8550).unit_costs,
        Cents::new(41_111 + 21_030 + 1_200)
    );
    assert_eq!(
        statement(&costs(), &UNIT_2, 0).unit_costs,
        Cents::new(82_346 + 21_030 + 1_200)
    );
}

#[test]
fn computes_prepayments_results_and_totals_per_unit() {
    let s1 = statement(&costs(), &UNIT_1, 8550);
    let s2 = statement(&costs(), &UNIT_2, 0);
    assert_eq!(s1.months, 12);
    assert_eq!(s1.prepayment, Cents::new(8550 * 12));
    assert_eq!(s1.result(), Cents::new(63_341 - 102_600));
    assert_eq!(s1.difference(), Cents::new(102_600 - 63_341));
    assert_eq!(s1.total_costs, Cents::new(123_457 + 50_000 + 9_999));
    assert_eq!(s1.unit_costs + s2.unit_costs, Cents::new(167_917));
    assert_eq!(s1.prepayment + s2.prepayment, Cents::new(102_600));
}

#[test]
fn recommends_a_new_prepayment_only_for_back_payments() {
    let s1 = statement(&costs(), &UNIT_1, 8550);
    let s2 = statement(&costs(), &UNIT_2, 0);
    assert_eq!(suggest_next_prepayment(Cents::new(8550), s1.result()), None);
    // 104576 / 12 = 8714.67 cents -> next 5-euro step 9000.
    assert_eq!(
        suggest_next_prepayment(Cents::ZERO, s2.result()),
        Some(Cents::new(9000))
    );
}

#[test]
fn skips_positions_without_total_and_disabled_ones() {
    let mut without_total = costs();
    without_total[0].total_amount = None;
    let with = statement(&costs(), &UNIT_1, 8550).unit_costs;
    let without = statement(&without_total, &UNIT_1, 8550).unit_costs;
    assert_eq!(without, with - mea_share(Cents::new(123_457), 333, 1000));
    // Disabled c5 (80 €) is neither in the unit costs nor in the total.
    let mut enabled_c5 = costs();
    enabled_c5[4].enabled = true;
    let s = statement(&enabled_c5, &UNIT_1, 0);
    assert_eq!(s.unit_costs, with + Cents::new(2_664));
    assert_eq!(s.total_costs, Cents::new(183_456 + 8_000));
}

#[test]
fn counts_review_progress_without_disabled_positions() {
    let reviews = [
        (true, true),
        (true, false),
        (true, true),
        (true, true),
        (false, false),
    ]
    .map(|(enabled, reviewed)| CostReview { enabled, reviewed });
    let progress = ReviewProgress::count(reviews, [true, false]);
    assert_eq!((progress.total, progress.reviewed), (6, 4));
    assert!(!progress.all_reviewed());
    assert_eq!(progress.unreviewed(), 2);
    assert!(!ReviewProgress::count([], []).all_reviewed());
    assert!(
        ReviewProgress::count(
            [CostReview {
                enabled: true,
                reviewed: true
            }],
            []
        )
        .all_reviewed()
    );
}

#[test]
fn keeps_zero_unit_amount_apart_from_empty() {
    // Port of the save-payload case: `Some(0)` is an entered zero, `None` is empty.
    let zero = position(Some(0), Some(0), true, PerAttachment);
    let empty_mea = position(Some(123_457), None, true, Mea);
    assert_eq!(zero.unit_share(&UNIT_1), Some(Cents::ZERO));
    assert_eq!(empty_mea.unit_share(&UNIT_1), Some(Cents::new(41_111)));
}
