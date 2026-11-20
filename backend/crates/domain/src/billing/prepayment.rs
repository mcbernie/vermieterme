//! Suggested new monthly prepayment after a billing period.

use crate::money::Cents;
use crate::rounding::{div_ceil, saturate_i64};

/// Step the suggestion is rounded up to: 5 euros.
pub const PREPAYMENT_STEP: Cents = Cents::new(500);

/// Recommended new monthly prepayment after a period that closes with a back
/// payment: the current monthly amount plus the back payment spread over twelve
/// months, rounded up to the next [`PREPAYMENT_STEP`].
///
/// `result` is costs minus prepayments ([`super::UnitStatement::result`]). A refund
/// or a balanced result gives `None` (no change suggested).
pub fn suggest_next_prepayment(current_monthly: Cents, result: Cents) -> Option<Cents> {
    if result.value() <= 0 {
        return None;
    }
    // ceil((monthly + result / 12) / step) * step, computed exactly as
    // ceil((monthly * 12 + result) / (12 * step)) * step.
    let step = i128::from(PREPAYMENT_STEP.value());
    let numerator = i128::from(current_monthly.value()) * 12 + i128::from(result.value());
    let steps = div_ceil(numerator, 12 * step)?;
    Some(Cents::new(saturate_i64(steps * step)))
}
