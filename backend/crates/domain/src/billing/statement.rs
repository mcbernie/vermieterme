//! Per-unit statement of a billing period: cost shares, prepayments and result.
//!
//! One function for the web detail, the tenant app and the PDF, so all three show
//! the same numbers (#69).

use super::key::DistributionKey;
use crate::date::CalendarDate;
use crate::money::Cents;
use crate::months::MonthRule;

/// One cost position of a billing period, seen from one unit.
///
/// The caller builds the positions per unit: `total_amount` is the property-wide
/// total, `unit_amount` the amount stored for **this** unit and position (decision
/// 29, table `cost_unit_amount`). Legacy data copied the single old amount to every
/// unit, so no special case is needed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CostPosition {
    /// Total amount of the property; `None` = no amount entered, the position is skipped.
    pub total_amount: Option<Cents>,
    /// Amount stored for this unit; `None` = not entered. Only used by the keys
    /// `per_notice` and `per_attachment`.
    pub unit_amount: Option<Cents>,
    /// Disabled positions count nowhere (#69).
    pub enabled: bool,
    /// The effective key (see [`super::effective_key`]).
    pub key: DistributionKey,
}

/// What a unit's share depends on besides the position itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitBasis {
    /// Co-ownership shares of the unit.
    pub shares: i64,
    /// Co-ownership shares of the whole property.
    pub total_shares: i64,
    /// Number of units of the property (key `per_unit`).
    pub unit_count: i64,
}

impl CostPosition {
    /// Whether the position takes part in the calculation (enabled and with a total).
    pub fn counts(&self) -> bool {
        self.enabled && self.total_amount.is_some()
    }

    /// The unit's share of this position, or `None` when the position does not count.
    ///
    /// `mea` and `per_unit` are always computed from the total and rounded per
    /// position to whole cents; a stored unit amount is ignored there, since it may
    /// be a stale value from before a key change (decision 5, #81). `per_notice` and
    /// `per_attachment` take the stored unit amount, 0 when none is stored.
    pub fn unit_share(&self, basis: &UnitBasis) -> Option<Cents> {
        let total = self.total_amount.filter(|_| self.enabled)?;
        Some(match self.key {
            DistributionKey::Mea => mea_share(total, basis.shares, basis.total_shares),
            DistributionKey::PerUnit => per_unit_share(total, basis.unit_count),
            DistributionKey::PerNotice | DistributionKey::PerAttachment => {
                self.unit_amount.unwrap_or(Cents::ZERO)
            }
        })
    }
}

/// Share of `total` for `shares` of `total_shares`, rounded half away from zero.
/// A property without shares gives 0 (as before).
pub fn mea_share(total: Cents, shares: i64, total_shares: i64) -> Cents {
    total.mul_ratio(shares, total_shares).unwrap_or(Cents::ZERO)
}

/// Equal share of `total` over `unit_count` units, rounded half away from zero.
/// A property without units gives 0.
pub fn per_unit_share(total: Cents, unit_count: i64) -> Cents {
    total.mul_ratio(1, unit_count).unwrap_or(Cents::ZERO)
}

/// Result of one unit in one billing period.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitStatement {
    /// Sum of the totals of all counting positions (whole property).
    pub total_costs: Cents,
    /// Sum of the unit's shares.
    pub unit_costs: Cents,
    /// Prepayment months of the period.
    pub months: u32,
    /// Monthly prepayment times months.
    pub prepayment: Cents,
}

impl UnitStatement {
    /// Prepayment minus costs: positive = refund to the tenant, negative = back
    /// payment (`difference` in the old totals and the tenant app).
    pub fn difference(&self) -> Cents {
        self.prepayment - self.unit_costs
    }

    /// Costs minus prepayment: positive = back payment (`result` on the web page,
    /// input of [`super::suggest_next_prepayment`]).
    pub fn result(&self) -> Cents {
        self.unit_costs - self.prepayment
    }
}

/// Billing period boundaries, both days inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Period {
    /// First day.
    pub start: CalendarDate,
    /// Last day.
    pub end: CalendarDate,
}

/// Computes the statement of a unit from the period's cost positions.
///
/// `costs` are the positions of the period built for this unit (see
/// [`CostPosition`]: total of the property, amount stored for this unit).
/// `monthly_prepayment` is the sum of the unit's monthly prepayments for this period.
pub fn unit_statement(
    costs: &[CostPosition],
    basis: &UnitBasis,
    monthly_prepayment: Cents,
    period: Period,
    rule: &dyn MonthRule,
) -> UnitStatement {
    let counting = || costs.iter().filter(|c| c.counts());
    let total_costs = counting().filter_map(|c| c.total_amount).sum();
    let unit_costs = counting().filter_map(|c| c.unit_share(basis)).sum();
    let months = rule.months(period.start, period.end);
    UnitStatement {
        total_costs,
        unit_costs,
        months,
        prepayment: monthly_prepayment.times(i64::from(months)),
    }
}
