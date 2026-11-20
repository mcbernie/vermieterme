//! Operating-cost billing (Betriebskostenabrechnung): distribution keys, unit
//! shares, prepayment months and results, prepayment suggestions, review progress.
//!
//! This is the only place with billing rules (project rule; formerly
//! `src/lib/billing.ts` and `src/features/billing/billing-calc.ts`).

mod key;
mod prepayment;
mod review;
mod statement;

pub use key::{DistributionKey, UnknownKey, effective_key};
pub use prepayment::{PREPAYMENT_STEP, suggest_next_prepayment};
pub use review::{CostReview, ReviewProgress};
pub use statement::{CostPosition, Period, ShareError, UnitBasis, UnitStatement, unit_statement};
