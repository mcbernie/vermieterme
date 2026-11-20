//! Billing data that keeps a unit in its property (decision 37, #117), and the unit
//! amounts that are cleared when it moves.

use std::collections::HashMap;

use sqlx::{AnyConnection, Row};
use vermieterme_domain::DistributionKey;

use super::super::{Rejection, cost_category};
use crate::error::{DbError, DbResult};
use crate::scope::AccountScope;

/// Prepares moving the unit `id` to another property, inside the caller's transaction
/// (property and unit already locked, lock order steps 1 and 2 of `repo::billing`).
///
/// Refused with [`Rejection::UnitInBillings`] while billings reference the unit: a
/// prepayment, or a unit amount of a position whose key in effect uses unit amounts
/// (#117). Otherwise its remaining unit amounts – kept from an earlier
/// `per_notice`/`per_attachment` key and ignored by the computation (decision 5) – are
/// deleted, so none stays attached to a position of the old property.
///
/// The categories of those amounts are locked first (step 4, sorted by id), before any
/// row below them is touched (step 5). A key changes only under that lock (the category
/// itself, or the override of a position, which a cost save writes under its category
/// lock), and new amounts or prepayments need the unit lock held here, so the decision
/// and the delete see one stable state. The reads after the locks are plain: the caller
/// runs under `READ COMMITTED` (the API with
/// `audited_read_committed`), so they see the latest commit.
pub(super) async fn detach_from_billings(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
) -> DbResult<Result<(), Rejection>> {
    let keys = lock_categories(conn, scope, id).await?;
    let prepayment = sqlx::query(
        "SELECT 1 AS used FROM prepayment WHERE account_id = ? AND unit_id = ? LIMIT 1",
    )
    .bind(scope.account_id())
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?;
    if prepayment.is_some() {
        return Ok(Err(Rejection::UnitInBillings));
    }
    let amounts = sqlx::query(
        "SELECT c.distribution_key_override, c.cost_category_id \
         FROM cost_unit_amount a JOIN cost c ON c.account_id = a.account_id AND c.id = a.cost_id \
         WHERE a.account_id = ? AND a.unit_id = ?",
    )
    .bind(scope.account_id())
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;
    for row in &amounts {
        let code = match row.try_get::<Option<String>, _>("distribution_key_override")? {
            Some(code) => Some(code),
            None => keys
                .get(&row.try_get::<String, _>("cost_category_id")?)
                .cloned(),
        };
        // A category the lock did not find counts as in use (cannot happen while
        // positions refer to it; refusing is the safe side).
        let Some(code) = code else {
            return Ok(Err(Rejection::UnitInBillings));
        };
        let key = DistributionKey::from_code(&code)
            .map_err(|_| DbError::InvalidValue("distribution_key"))?;
        if key.uses_unit_amounts() {
            return Ok(Err(Rejection::UnitInBillings));
        }
    }
    if !amounts.is_empty() {
        sqlx::query("DELETE FROM cost_unit_amount WHERE account_id = ? AND unit_id = ?")
            .bind(scope.account_id())
            .bind(id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(Ok(()))
}

/// Locks the categories of the positions holding unit amounts of the unit, sorted by id,
/// and returns their key codes by id. The ids come from a plain read: the category of a
/// position never changes, and new amounts need the unit lock the caller holds.
async fn lock_categories(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    unit_id: &str,
) -> DbResult<HashMap<String, String>> {
    let rows = sqlx::query(
        "SELECT DISTINCT c.cost_category_id FROM cost_unit_amount a \
         JOIN cost c ON c.account_id = a.account_id AND c.id = a.cost_id \
         WHERE a.account_id = ? AND a.unit_id = ? ORDER BY c.cost_category_id",
    )
    .bind(scope.account_id())
    .bind(unit_id)
    .fetch_all(&mut *conn)
    .await?;
    let mut keys = HashMap::new();
    for row in &rows {
        let id: String = row.try_get("cost_category_id")?;
        match cost_category::lock(conn, scope, &id).await {
            Ok(category) => {
                keys.insert(id, category.distribution_key);
            }
            Err(DbError::NotFound) => {}
            Err(other) => return Err(other),
        }
    }
    Ok(keys)
}
