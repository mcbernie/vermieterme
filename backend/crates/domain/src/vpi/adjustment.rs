//! Rent adjustment proposal for index rents (Indexmiete).
//!
//! Formerly inline in `src/app/api/vpi/suggestions/route.ts`; the API crate loads
//! tenants, rents and index values and calls [`index_adjustment`] per tenant.

use super::VpiValue;
use crate::date::CalendarDate;
use crate::money::Cents;
use crate::rounding::{div_round_half_away, saturate_i64};

/// Proposed adjustment of one index rent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexAdjustment {
    /// Index change since the reference value in basis points (hundredths of a
    /// percent, `250` = +2.50 %), rounded half away from zero.
    pub percentage_change_basis_points: i64,
    /// Current rent scaled by `current / reference`, rounded to whole cents.
    pub suggested_rent: Cents,
    /// Calendar months since the last adjustment (or move-in).
    pub months_since_last_adjustment: i64,
    /// Whether the minimum interval between adjustments has passed.
    pub eligible: bool,
}

/// Calendar months from `reference` to `today`, ignoring the day of month (as in
/// the old code); negative when `reference` lies in the future.
pub fn months_since(reference: CalendarDate, today: CalendarDate) -> i64 {
    today.month_index() - reference.month_index()
}

/// Computes the proposal for a tenant with an index lease.
///
/// * `current_rent` – latest rent of the unit; 0 gives `None` (no proposal, as before).
/// * `reference` / `current` – index at the last adjustment and the latest index of
///   the same base year; a reference of 0 gives `None`.
/// * `reference_date` – date of the last adjustment, or the move-in date without one.
/// * `today` – passed in so the function stays free of clock access.
/// * `min_months` – minimum months between adjustments from the lease.
pub fn index_adjustment(
    current_rent: Cents,
    reference: VpiValue,
    current: VpiValue,
    reference_date: CalendarDate,
    today: CalendarDate,
    min_months: i64,
) -> Option<IndexAdjustment> {
    if current_rent == Cents::ZERO || reference.tenths() == 0 {
        return None;
    }
    let reference_tenths = i128::from(reference.tenths());
    let change = i128::from(current.tenths()) - reference_tenths;
    let basis_points = div_round_half_away(change * 10_000, reference_tenths)?;
    let suggested_rent = current_rent.mul_ratio(current.tenths(), reference.tenths())?;
    let months = months_since(reference_date, today);
    Some(IndexAdjustment {
        percentage_change_basis_points: saturate_i64(basis_points),
        suggested_rent,
        months_since_last_adjustment: months,
        eligible: months >= min_months,
    })
}
