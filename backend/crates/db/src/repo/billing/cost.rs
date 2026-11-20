//! Cost positions of a billing period: one per cost category, partial updates with an
//! optimistic version (#25, #67).

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};
use vermieterme_domain::DistributionKey;
use vermieterme_domain::billing::effective_key;

use super::period;
use super::{Checked, Rejection, Saved};
use crate::error::{DbError, DbResult};
use crate::repo::masterdata::cost_category;
use crate::row::{flag, flag_value, new_id, now_millis};
use crate::scope::AccountScope;
use crate::tx::in_transaction_read_committed;

pub(super) use super::cost_unit::insert_unit_amount;
use super::cost_unit::with_unit_amounts;
pub use super::cost_unit::{UnitAmountSaved, set_unit_amount};

/// Amount stored for one unit of a position (`per_notice` / `per_attachment`, #80).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitAmount {
    /// The unit.
    pub unit_id: String,
    /// Amount in cents.
    pub amount_cents: i64,
}

/// A cost position with its category and the amounts stored per unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cost {
    /// Primary key.
    pub id: String,
    /// The period.
    pub billing_period_id: String,
    /// The category.
    pub cost_category_id: String,
    /// Name of the category.
    pub category_name: String,
    /// Default key of the category.
    pub category_key: DistributionKey,
    /// Key chosen for this period only.
    pub override_key: Option<DistributionKey>,
    /// Total of the property; `None` = no amount entered (#25).
    pub total_amount_cents: Option<i64>,
    /// Stored amounts per unit, sorted by unit id.
    pub unit_amounts: Vec<UnitAmount>,
    /// Disabled positions count nowhere.
    pub enabled: bool,
    /// Whether the position has been reviewed.
    pub reviewed: bool,
    /// Optimistic lock, raised by every change.
    pub version: i64,
    /// Creation time (Unix ms).
    pub created_at: i64,
    /// Last change (Unix ms).
    pub updated_at: i64,
}

impl Cost {
    /// The key in effect: the override wins over the category default.
    pub fn key(&self) -> DistributionKey {
        effective_key(self.override_key, self.category_key)
    }

    /// The amount stored for `unit_id`, if any.
    pub fn unit_amount(&self, unit_id: &str) -> Option<i64> {
        self.unit_amounts
            .iter()
            .find(|a| a.unit_id == unit_id)
            .map(|a| a.amount_cents)
    }
}

/// Changes of a position; `None` keeps a value, `Some(None)` clears a nullable one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CostPatch {
    /// New total (`Some(None)` = no amount).
    pub total_amount_cents: Option<Option<i64>>,
    /// Enable or disable.
    pub enabled: Option<bool>,
    /// New key override (`Some(None)` = category default).
    pub override_key: Option<Option<DistributionKey>>,
    /// Set the review state explicitly.
    pub reviewed: Option<bool>,
    /// Version the client last saw; outdated -> [`Rejection::CostVersion`].
    pub version: Option<i64>,
}

/// Select list and join read by [`from_row`].
macro_rules! select {
    () => {
        "SELECT c.id, c.billing_period_id, c.cost_category_id, cc.name AS category_name, \
         cc.distribution_key AS category_key, c.total_amount_cents, c.reviewed, c.enabled, \
         c.distribution_key_override, c.version, c.created_at, c.updated_at FROM cost c \
         JOIN cost_category cc ON cc.account_id = c.account_id AND cc.id = c.cost_category_id \
         WHERE c.account_id = ? AND c.billing_period_id = ?"
    };
}

/// Parses a stored key code.
fn key_of(code: &str) -> DbResult<DistributionKey> {
    DistributionKey::from_code(code).map_err(|_| DbError::InvalidValue("distribution_key"))
}

/// Maps a row selected with `select!()` (without unit amounts).
fn from_row(row: &AnyRow) -> DbResult<Cost> {
    let override_code: Option<String> = row.try_get("distribution_key_override")?;
    Ok(Cost {
        id: row.try_get("id")?,
        billing_period_id: row.try_get("billing_period_id")?,
        cost_category_id: row.try_get("cost_category_id")?,
        category_name: row.try_get("category_name")?,
        category_key: key_of(&row.try_get::<String, _>("category_key")?)?,
        override_key: override_code.as_deref().map(key_of).transpose()?,
        total_amount_cents: row.try_get("total_amount_cents")?,
        unit_amounts: Vec::new(),
        enabled: flag(row, "enabled")?,
        reviewed: flag(row, "reviewed")?,
        version: row.try_get("version")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

/// Lists the positions of a period by category order; an unknown or foreign period is
/// [`DbError::NotFound`].
pub async fn list(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    period_id: &str,
) -> DbResult<Vec<Cost>> {
    period::get(conn, scope, period_id).await?;
    let rows = sqlx::query(concat!(
        select!(),
        " ORDER BY cc.sort_order, cc.name, cc.id"
    ))
    .bind(scope.account_id())
    .bind(period_id)
    .fetch_all(&mut *conn)
    .await?;
    let costs = rows.iter().map(from_row).collect::<DbResult<Vec<_>>>()?;
    with_unit_amounts(conn, scope, period_id, costs).await
}

/// The position of `category_id` in a period, if it exists.
pub(super) async fn find(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    period_id: &str,
    category_id: &str,
) -> DbResult<Option<Cost>> {
    let row = sqlx::query(concat!(select!(), " AND c.cost_category_id = ?"))
        .bind(scope.account_id())
        .bind(period_id)
        .bind(category_id)
        .fetch_optional(&mut *conn)
        .await?;
    let Some(row) = row else { return Ok(None) };
    let cost = from_row(&row)?;
    let mut costs = with_unit_amounts(conn, scope, period_id, vec![cost]).await?;
    Ok(costs.pop())
}

/// A locked period and category with the position of the category, if it exists.
pub(super) struct Position {
    /// The period.
    pub period: period::BillingPeriod,
    /// Default key of the category (in effect while there is no position).
    pub category_key: DistributionKey,
    /// The position.
    pub cost: Option<Cost>,
}

/// Locks the period and the category (lock order steps 3 and 4) and reads the position
/// of the category, if it exists.
pub(super) async fn lock_position(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    period_id: &str,
    category_id: &str,
) -> DbResult<Position> {
    let period = period::lock(conn, scope, period_id).await?;
    let category = cost_category::lock(conn, scope, category_id).await?;
    let cost = find(conn, scope, period_id, category_id).await?;
    Ok(Position {
        period,
        category_key: key_of(&category.distribution_key)?,
        cost,
    })
}

/// Inserts a position with the values of `patch` (defaults: enabled, reviewed, no
/// amount; a new entry counts as reviewed like before) and returns it.
pub(super) async fn insert(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    period_id: &str,
    category_id: &str,
    patch: &CostPatch,
) -> DbResult<Cost> {
    let now = now_millis();
    sqlx::query(
        "INSERT INTO cost (id, account_id, billing_period_id, cost_category_id, \
         total_amount_cents, reviewed, enabled, distribution_key_override, version, \
         created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?)",
    )
    .bind(new_id())
    .bind(scope.account_id())
    .bind(period_id)
    .bind(category_id)
    .bind(patch.total_amount_cents.flatten())
    .bind(flag_value(patch.reviewed.unwrap_or(true)))
    .bind(flag_value(patch.enabled.unwrap_or(true)))
    .bind(patch.override_key.flatten().map(DistributionKey::code))
    .bind(now)
    .bind(now)
    .execute(&mut *conn)
    .await?;
    find(conn, scope, period_id, category_id)
        .await?
        .ok_or(DbError::NotFound)
}

/// Saves the sent fields of the position of `category_id`, creating it if needed.
///
/// `reviewed` follows the request if sent; otherwise it turns `false` only when the
/// total really changes (#67). A request that changes nothing writes nothing and keeps
/// the version. Changes are one `UPDATE` of the sent fields under the category lock, so
/// two parallel saves of different fields both survive (#25).
pub async fn save(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    period_id: &str,
    category_id: &str,
    patch: &CostPatch,
) -> Checked<Saved<Cost>> {
    in_transaction_read_committed(conn, async |tx| {
        let position = lock_position(tx, scope, period_id, category_id).await?;
        let Some(current) = position.cost else {
            let cost = insert(tx, scope, period_id, category_id, patch).await?;
            return Ok(Ok(Saved::Created(cost)));
        };
        if patch.version.is_some_and(|v| v != current.version) {
            return Ok(Err(Rejection::CostVersion(Box::new(current))));
        }
        let amount_changed = patch
            .total_amount_cents
            .is_some_and(|total| total != current.total_amount_cents);
        let reviewed = patch
            .reviewed
            .unwrap_or(current.reviewed && !amount_changed);
        let unchanged = !amount_changed
            && patch.enabled.is_none_or(|e| e == current.enabled)
            && patch.override_key.is_none_or(|k| k == current.override_key)
            && reviewed == current.reviewed;
        if unchanged {
            return Ok(Ok(Saved::Updated(current)));
        }
        sqlx::query(
            "UPDATE cost SET \
             total_amount_cents = CASE WHEN ? THEN ? ELSE total_amount_cents END, \
             enabled = COALESCE(?, enabled), \
             distribution_key_override = CASE WHEN ? THEN ? ELSE distribution_key_override END, \
             reviewed = ?, version = version + 1, updated_at = ? \
             WHERE account_id = ? AND id = ? AND version = ?",
        )
        .bind(flag_value(patch.total_amount_cents.is_some()))
        .bind(patch.total_amount_cents.flatten())
        .bind(patch.enabled.map(flag_value))
        .bind(flag_value(patch.override_key.is_some()))
        .bind(patch.override_key.flatten().map(DistributionKey::code))
        .bind(flag_value(reviewed))
        .bind(now_millis())
        .bind(scope.account_id())
        .bind(&current.id)
        .bind(current.version)
        .execute(&mut *tx)
        .await?;
        let saved = find(tx, scope, period_id, category_id)
            .await?
            .ok_or(DbError::NotFound)?;
        Ok(Ok(Saved::Updated(saved)))
    })
    .await
}
