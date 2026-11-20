//! Monthly prepayments of the units in a billing period: one per unit, partial updates
//! with an optimistic version (#25, #67).

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};

use super::{Checked, Rejection, Saved, period};
use crate::error::{DbError, DbResult};
use crate::repo::masterdata::unit;
use crate::row::{flag, flag_value, new_id, now_millis};
use crate::scope::AccountScope;
use crate::tx::in_transaction;

/// The monthly prepayment of one unit in one period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepayment {
    /// Primary key.
    pub id: String,
    /// The period.
    pub billing_period_id: String,
    /// The unit.
    pub unit_id: String,
    /// Monthly amount in cents.
    pub monthly_amount_cents: i64,
    /// Whether the prepayment has been reviewed.
    pub reviewed: bool,
    /// Optimistic lock, raised by every change.
    pub version: i64,
    /// Creation time (Unix ms).
    pub created_at: i64,
    /// Last change (Unix ms).
    pub updated_at: i64,
}

/// Changes of a prepayment; `None` keeps a value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrepaymentPatch {
    /// New monthly amount (required when the prepayment is new).
    pub monthly_amount_cents: Option<i64>,
    /// Set the review state explicitly.
    pub reviewed: Option<bool>,
    /// Version the client last saw; outdated -> [`Rejection::PrepaymentVersion`].
    pub version: Option<i64>,
}

/// Column list read by [`from_row`].
macro_rules! columns {
    () => {
        "id, billing_period_id, unit_id, monthly_amount_cents, reviewed, version, created_at, \
         updated_at"
    };
}

/// Maps a row selected with `columns!()`.
fn from_row(row: &AnyRow) -> DbResult<Prepayment> {
    Ok(Prepayment {
        id: row.try_get("id")?,
        billing_period_id: row.try_get("billing_period_id")?,
        unit_id: row.try_get("unit_id")?,
        monthly_amount_cents: row.try_get("monthly_amount_cents")?,
        reviewed: flag(row, "reviewed")?,
        version: row.try_get("version")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

/// Lists the prepayments of a period by unit; an unknown or foreign period is
/// [`DbError::NotFound`].
pub async fn list(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    period_id: &str,
) -> DbResult<Vec<Prepayment>> {
    period::get(conn, scope, period_id).await?;
    let rows = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM prepayment WHERE account_id = ? AND billing_period_id = ? ORDER BY unit_id"
    ))
    .bind(scope.account_id())
    .bind(period_id)
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(from_row).collect()
}

/// The prepayment of `unit_id` in a period, if it exists.
async fn find(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    period_id: &str,
    unit_id: &str,
) -> DbResult<Option<Prepayment>> {
    let row = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM prepayment WHERE account_id = ? AND billing_period_id = ? AND unit_id = ?"
    ))
    .bind(scope.account_id())
    .bind(period_id)
    .bind(unit_id)
    .fetch_optional(&mut *conn)
    .await?;
    row.as_ref().map(from_row).transpose()
}

/// Saves the sent fields of the prepayment of `unit_id`, creating it if needed (then a
/// monthly amount is required: [`Rejection::AmountRequired`]).
///
/// The unit must belong to the period's property, otherwise it is
/// [`DbError::NotFound`]; its row is locked first (decision 37), so moving the unit
/// cannot slip in between. `reviewed` and the version behave like for costs: `reviewed`
/// turns `false` only when the amount really changes, a request that changes nothing
/// writes nothing.
pub async fn save(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    period_id: &str,
    unit_id: &str,
    patch: &PrepaymentPatch,
) -> Checked<Saved<Prepayment>> {
    in_transaction(conn, async |tx| {
        let found = period::lock(tx, scope, period_id).await?;
        let locked = unit::lock(tx, scope, unit_id).await?;
        if locked.property_id != found.property_id {
            return Err(DbError::NotFound);
        }
        let Some(current) = find(tx, scope, period_id, unit_id).await? else {
            let Some(amount) = patch.monthly_amount_cents else {
                return Ok(Err(Rejection::AmountRequired));
            };
            insert(tx, scope, period_id, unit_id, amount, patch.reviewed).await?;
            let created = find(tx, scope, period_id, unit_id).await?;
            return created
                .ok_or(DbError::NotFound)
                .map(|p| Ok(Saved::Created(p)));
        };
        if patch.version.is_some_and(|v| v != current.version) {
            return Ok(Err(Rejection::PrepaymentVersion(Box::new(current))));
        }
        let amount_changed = patch
            .monthly_amount_cents
            .is_some_and(|amount| amount != current.monthly_amount_cents);
        let reviewed = patch
            .reviewed
            .unwrap_or(current.reviewed && !amount_changed);
        if !amount_changed && reviewed == current.reviewed {
            return Ok(Ok(Saved::Updated(current)));
        }
        sqlx::query(
            "UPDATE prepayment SET monthly_amount_cents = COALESCE(?, monthly_amount_cents), \
             reviewed = ?, version = version + 1, updated_at = ? \
             WHERE account_id = ? AND id = ? AND version = ?",
        )
        .bind(patch.monthly_amount_cents)
        .bind(flag_value(reviewed))
        .bind(now_millis())
        .bind(scope.account_id())
        .bind(&current.id)
        .bind(current.version)
        .execute(&mut *tx)
        .await?;
        let saved = find(tx, scope, period_id, unit_id).await?;
        saved
            .ok_or(DbError::NotFound)
            .map(|p| Ok(Saved::Updated(p)))
    })
    .await
}

/// Inserts a prepayment (a new entry counts as reviewed unless sent otherwise, as before).
async fn insert(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    period_id: &str,
    unit_id: &str,
    amount: i64,
    reviewed: Option<bool>,
) -> DbResult<()> {
    let now = now_millis();
    sqlx::query(
        "INSERT INTO prepayment (id, account_id, billing_period_id, unit_id, \
         monthly_amount_cents, reviewed, version, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?)",
    )
    .bind(new_id())
    .bind(scope.account_id())
    .bind(period_id)
    .bind(unit_id)
    .bind(amount)
    .bind(flag_value(reviewed.unwrap_or(true)))
    .bind(now)
    .bind(now)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
