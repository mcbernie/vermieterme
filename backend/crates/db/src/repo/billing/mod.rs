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
//!
//! ## Lock order
//!
//! The one place this order is written down; master data (`unit`, `property`) follows
//! it too. A change may skip steps but never takes an earlier step after a later one,
//! so two changes never wait for each other in a circle (#113):
//!
//! 1. **property** – creating, changing or deleting a period (against overlapping
//!    periods), creating, moving (current and target property, sorted by id) or deleting
//!    a unit, deleting a property; moving a tenant to another unit (the properties of
//!    the current and the target unit, sorted by id, #120); storing a document or
//!    collecting the files of a cascading delete (the property of the assignment, #106);
//! 2. **units** – all units of the property (creating a period, deleting a property) or
//!    the one unit (moving it, and before inserting a prepayment or a unit amount,
//!    decision 37); for a tenant move both units (sorted by id), then the **tenant**
//!    row; for a document of a tenant its unit and tenant row in one statement. A
//!    tenant edit that keeps the unit takes no lock before its `UPDATE` (which locks
//!    only the tenant row) and binds that `UPDATE` to the unit it read (a concurrent
//!    move -> conflict, #122);
//! 3. **billing period** – the changed one or the copy source;
//! 4. **cost category** – deleting a category locks it too, so a parallel cost save
//!    gives 409 or 404, never a foreign-key error; moving a unit locks the categories
//!    of its unit amounts (sorted by id) before it checks and deletes them (#117);
//! 5. rows below (costs, unit amounts, prepayments), by the writes themselves.
//!
//! Outside these steps, storing a document locks its **account** row last, after its
//! steps 1 and 2 (storage quota, #124); no change locks the account row before a step
//! above.
//!
//! Where a step needs an id from a later row (the property of a period or unit), that id
//! comes from a plain read first. If a later row turns out to have moved to a property
//! that is not locked, the change is refused with a conflict (409, retry) instead of
//! taking that property's lock late.
//!
//! ## No range locks beyond the property (#116)
//!
//! Every change here runs under `READ COMMITTED` on MySQL
//! ([`in_transaction_read_committed`](crate::in_transaction_read_committed); the API
//! starts its outer transaction with it too). InnoDB then takes no gap or next-key
//! locks for searches, scans and cascades, and plain reads see the latest commit. Under
//! the default `REPEATABLE READ` a locking range read (`… property_id = ? … FOR
//! UPDATE`) or a cascade also locks the first row *behind* its range – a row of the
//! neighbouring property or account in the index – and two changes of different
//! properties could wait for each other in a circle. Hence:
//!
//! - checks after the locks (the overlap of periods) are plain reads, never locking
//!   range reads;
//! - only rows of the locked property itself are locked, by key or by its own range.
//!
//! Business rules that refuse a change come back as a [`Rejection`] inside `Ok`, so the
//! HTTP layer can answer precisely while database failures stay `DbError`s.

use vermieterme_domain::DistributionKey;

use crate::error::DbResult;

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
mod overlap;
pub mod period;
pub mod prepayment;
pub mod summary;

pub use cost::{Cost, CostPatch, UnitAmount, UnitAmountSaved};
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
    /// A unit amount was sent for a position whose key in effect computes the shares
    /// from the total (`mea`, `per_unit`); carries that key.
    UnitAmountKey(DistributionKey),
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
