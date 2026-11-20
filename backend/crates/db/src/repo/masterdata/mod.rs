//! Master data of an account: properties, units, tenants, rent changes, cost categories,
//! the landlord, the PDF template and the global VPI values (P4).
//!
//! Every function with account reference takes an [`AccountScope`](crate::AccountScope)
//! and filters on it, so a foreign id is [`DbError::NotFound`](crate::DbError::NotFound)
//! like a missing one. Child rows are written with the scope's account id and the
//! composite foreign keys of the schema, so a parent of another account cannot be
//! referenced either.
//!
//! Rules that depend on other rows (unit shares against the property, categories in use)
//! are checked inside the transaction of the change and reported as a [`Rejection`]
//! inside `Ok`, so the HTTP layer can answer with a precise message (400/409) while
//! database failures stay [`DbError`](crate::DbError)s.

use crate::error::DbResult;

pub mod cost_category;
pub mod landlord;
pub mod pdf_template;
pub mod property;
pub mod rent_change;
pub mod suggestions;
pub mod tenant;
pub mod unit;
pub mod vpi;

/// A change refused because of a business rule (nothing was written).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    /// The shares of a unit exceed the total shares of its property.
    SharesAboveTotal,
    /// The new total shares of a property are below the shares of one of its units.
    TotalBelowUnitShares,
    /// The cost category is used by costs of a billing period.
    CategoryInUse,
}

/// Result of a change that a business rule can refuse.
pub type Checked<T> = DbResult<Result<T, Rejection>>;
