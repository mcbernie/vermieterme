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
///
/// Plain stored data, deliberately unchecked: a property whose shares are
/// inconsistent can still be billed as long as no position needs them. The checks
/// happen in [`UnitBasis::mea_share`] and [`UnitBasis::per_unit_share`], the only
/// way to compute these shares. The API validates on write (`shares >= 0`,
/// `totalShares >= 1`); legacy properties may have `total_shares = 0` (the old form
/// stored 0 for an empty field), which is accepted when the unit has 0 shares too
/// and gives MEA shares of 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitBasis {
    /// Co-ownership shares of the unit.
    pub shares: i64,
    /// Co-ownership shares of the whole property.
    pub total_shares: i64,
    /// Number of units of the property (key `per_unit`).
    pub unit_count: i64,
}

/// A share cannot be computed because the stored basis is inconsistent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ShareError {
    /// Key `mea`: unit shares negative or above the shares of the property.
    #[error(
        "MEA-Anteil nicht berechenbar: Die Miteigentumsanteile der Einheit sind negativ oder größer als die des Objekts"
    )]
    InvalidShares,
    /// Key `per_unit`: the property has no units.
    #[error("Anteil nach Einheiten nicht berechenbar: Das Objekt hat keine Einheiten")]
    NoUnits,
}

impl UnitBasis {
    /// Checks the shares without computing anything: `0 <= shares <= total_shares`.
    /// The rule of [`UnitBasis::mea_share`], usable for validation on write and for
    /// reports on legacy data.
    pub fn check_shares(&self) -> Result<(), ShareError> {
        if self.shares < 0 || self.shares > self.total_shares {
            return Err(ShareError::InvalidShares);
        }
        Ok(())
    }

    /// Checks the unit count without computing anything: `unit_count >= 1`.
    /// The rule of [`UnitBasis::per_unit_share`].
    pub fn check_unit_count(&self) -> Result<(), ShareError> {
        if self.unit_count < 1 {
            return Err(ShareError::NoUnits);
        }
        Ok(())
    }

    /// MEA share of `total`: `total * shares / total_shares`, rounded half away from
    /// zero. Requires [`UnitBasis::check_shares`]; `total_shares = 0` (with
    /// `shares = 0`) gives 0.
    pub fn mea_share(&self, total: Cents) -> Result<Cents, ShareError> {
        self.check_shares()?;
        // `None` only for total_shares = 0, which the check allows with shares = 0.
        Ok(total
            .mul_ratio(self.shares, self.total_shares)
            .unwrap_or(Cents::ZERO))
    }

    /// Equal share of `total` over all units of the property, rounded half away from
    /// zero. Requires [`UnitBasis::check_unit_count`].
    pub fn per_unit_share(&self, total: Cents) -> Result<Cents, ShareError> {
        self.check_unit_count()?;
        // Unreachable `None`: the check guarantees unit_count >= 1.
        Ok(total.mul_ratio(1, self.unit_count).unwrap_or(Cents::ZERO))
    }
}

impl CostPosition {
    /// Whether the position takes part in the calculation (enabled and with a total).
    pub fn counts(&self) -> bool {
        self.enabled && self.total_amount.is_some()
    }

    /// The unit's share of this position, `Ok(None)` when the position does not count.
    ///
    /// `mea` and `per_unit` are always computed from the total and rounded per
    /// position to whole cents; a stored unit amount is ignored there, since it may
    /// be a stale value from before a key change (decision 5, #81). `per_notice` and
    /// `per_attachment` take the stored unit amount, 0 when none is stored, and never
    /// look at the basis. Errors only when a computed key meets an inconsistent basis.
    pub fn unit_share(&self, basis: &UnitBasis) -> Result<Option<Cents>, ShareError> {
        let Some(total) = self.total_amount.filter(|_| self.enabled) else {
            return Ok(None);
        };
        let share = match self.key {
            DistributionKey::Mea => basis.mea_share(total)?,
            DistributionKey::PerUnit => basis.per_unit_share(total)?,
            DistributionKey::PerNotice | DistributionKey::PerAttachment => {
                self.unit_amount.unwrap_or(Cents::ZERO)
            }
        };
        Ok(Some(share))
    }
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
/// Fails only if a counting `mea`/`per_unit` position meets an inconsistent basis.
pub fn unit_statement(
    costs: &[CostPosition],
    basis: &UnitBasis,
    monthly_prepayment: Cents,
    period: Period,
    rule: &dyn MonthRule,
) -> Result<UnitStatement, ShareError> {
    let counting = || costs.iter().filter(|c| c.counts());
    let total_costs = counting().filter_map(|c| c.total_amount).sum();
    let mut unit_costs = Cents::ZERO;
    for cost in counting() {
        unit_costs += cost.unit_share(basis)?.unwrap_or(Cents::ZERO);
    }
    let months = rule.months(period.start, period.end);
    Ok(UnitStatement {
        total_costs,
        unit_costs,
        months,
        prepayment: monthly_prepayment.times(i64::from(months)),
    })
}
