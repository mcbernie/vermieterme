//! Billing of an account (P5): billing periods with copy, cost positions with amounts per
//! unit, and prepayments.
//!
//! Every function takes an [`AccountScope`](crate::AccountScope); a foreign id is
//! [`DbError::NotFound`](crate::DbError::NotFound) like a missing one. Units and cost
//! categories referenced by a request must belong to the account **and** to the property
//! of the period, otherwise they are not found either.
//!
//! # Concurrency
//!
//! Cost and prepayment writes are partial (only the sent fields change, #25) and carry an
//! optimistic version (#67). Every change runs in one transaction and locks the rows it
//! depends on (MySQL `FOR UPDATE`; SQLite holds the write lock from `BEGIN IMMEDIATE`).
//! The lock order is the same everywhere, so two changes never wait for each other in a
//! circle:
//!
//! 1. billing period (the changed one, or the copy source),
//! 2. cost category (deleting a category locks it too, so a parallel cost save gives 409
//!    or 404, never a foreign-key error),
//! 3. unit (before inserting a prepayment or a unit amount; moving a unit locks it too,
//!    decision 37),
//! 4. property (creating or moving a period, against overlapping periods).
//!
//! Business rules that refuse a change come back as a [`Rejection`] inside `Ok`, so the
//! HTTP layer can answer precisely while database failures stay `DbError`s.

use sqlx::{AnyConnection, Row};

use crate::dialect::Dialect;
use crate::error::DbResult;
use crate::scope::AccountScope;

/// Column list of `billing_period` read by `period::from_row` (period and summary
/// modules; defined before them so both can expand it).
macro_rules! period_columns {
    () => {
        "id, property_id, start_date, end_date, billing_date, sent_date, paid_date, \
         copied_from_id, created_at, updated_at"
    };
}

mod copy;
pub mod cost;
mod cost_unit;
pub mod period;
pub mod prepayment;
pub mod summary;

pub use cost::{Cost, CostPatch, UnitAmount};
pub use period::{BillingPeriod, NewPeriod, PeriodPatch};
pub use prepayment::{Prepayment, PrepaymentPatch};
pub use summary::PeriodSummary;

/// A billing change refused because of a business rule (nothing was written).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejection {
    /// Another period of the property overlaps; carries its first and last day.
    Overlap {
        /// First day of the overlapping period (`YYYY-MM-DD`).
        start: String,
        /// Last day of the overlapping period (`YYYY-MM-DD`).
        end: String,
    },
    /// The last day is not after the first day (checked against stored dates on update).
    EndNotAfterStart,
    /// The property has no units, so there is nothing to bill.
    NoUnits,
    /// The period to copy from does not exist in the account.
    SourceNotFound,
    /// The period to copy from belongs to another property.
    SourceOtherProperty,
    /// A new prepayment needs a monthly amount.
    AmountRequired,
    /// The sent version of a cost is outdated; carries the current state.
    CostVersion(Box<Cost>),
    /// The sent version of a prepayment is outdated; carries the current state.
    PrepaymentVersion(Box<Prepayment>),
}

/// Result of a change that a business rule can refuse.
pub type Checked<T> = DbResult<Result<T, Rejection>>;

/// Outcome of a save that creates the row if it does not exist yet (201 vs. 200).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Saved<T> {
    /// The row was created.
    Created(T),
    /// The row existed (and was changed, or was already in the requested state).
    Updated(T),
}

impl<T> Saved<T> {
    /// The saved row.
    pub fn value(&self) -> &T {
        match self {
            Saved::Created(value) | Saved::Updated(value) => value,
        }
    }
}

/// Ids of the units of a property, locked on MySQL until the end of the transaction
/// (lock order step 3), sorted by id so parallel callers lock in the same order.
async fn lock_property_units(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    property_id: &str,
) -> DbResult<Vec<String>> {
    let sql = Dialect::of(conn)?.pick(
        "SELECT id FROM unit WHERE account_id = ? AND property_id = ? ORDER BY id",
        "SELECT id FROM unit WHERE account_id = ? AND property_id = ? ORDER BY id FOR UPDATE",
    );
    let rows = sqlx::query(sql)
        .bind(scope.account_id())
        .bind(property_id)
        .fetch_all(&mut *conn)
        .await?;
    rows.iter()
        .map(|row| Ok(row.try_get::<String, _>("id")?))
        .collect()
}
