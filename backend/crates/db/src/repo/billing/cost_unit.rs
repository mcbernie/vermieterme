//! The amount of one unit in a cost position (`per_notice` / `per_attachment`, decision
//! 29, #80).

use sqlx::{AnyConnection, Row};

use super::cost::{self, Cost, CostPatch, UnitAmount};
use super::{Checked, Rejection};
use crate::error::{DbError, DbResult};
use crate::repo::masterdata::unit;
use crate::row::now_millis;
use crate::scope::AccountScope;
use crate::tx::in_transaction;

/// Sets (`Some`) or clears (`None`) the amount of `unit_id` in the position of
/// `category_id`; the position is created (without total) if it does not exist yet.
///
/// The unit must belong to the period's property, otherwise it is
/// [`DbError::NotFound`]. Its row is locked before writing (decision 37), so moving the
/// unit to another property cannot slip in between. The version of the position guards
/// like for [`cost::save`]; a real change of the amount raises it and makes the position
/// unreviewed (#67), an unchanged amount writes nothing.
pub async fn set_unit_amount(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    (period_id, category_id, unit_id): (&str, &str, &str),
    amount_cents: Option<i64>,
    version: Option<i64>,
) -> Checked<Cost> {
    in_transaction(conn, async |tx| {
        let (found, existing) = cost::lock_position(tx, scope, period_id, category_id).await?;
        let locked = unit::lock(tx, scope, unit_id).await?;
        if locked.property_id != found.property_id {
            return Err(DbError::NotFound);
        }
        let (current, created) = match existing {
            Some(current) => (current, false),
            None => {
                let empty = CostPatch::default();
                let cost = cost::insert(tx, scope, period_id, category_id, &empty).await?;
                (cost, true)
            }
        };
        if !created && version.is_some_and(|v| v != current.version) {
            return Ok(Err(Rejection::CostVersion(Box::new(current))));
        }
        if current.unit_amount(unit_id) == amount_cents {
            return Ok(Ok(current));
        }
        write_amount(tx, scope, &current.id, unit_id, amount_cents).await?;
        if !created {
            touch(tx, scope, &current).await?;
        }
        cost::find(tx, scope, period_id, category_id)
            .await?
            .ok_or(DbError::NotFound)
            .map(Ok)
    })
    .await
}

/// Replaces the stored amount: delete, then insert unless cleared.
async fn write_amount(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    cost_id: &str,
    unit_id: &str,
    amount_cents: Option<i64>,
) -> DbResult<()> {
    sqlx::query(
        "DELETE FROM cost_unit_amount WHERE account_id = ? AND cost_id = ? AND unit_id = ?",
    )
    .bind(scope.account_id())
    .bind(cost_id)
    .bind(unit_id)
    .execute(&mut *conn)
    .await?;
    if let Some(amount) = amount_cents {
        cost::insert_unit_amount(conn, scope, cost_id, unit_id, amount).await?;
    }
    Ok(())
}

/// Raises the version of the position and marks it unreviewed (an amount changed).
async fn touch(conn: &mut AnyConnection, scope: &AccountScope, current: &Cost) -> DbResult<()> {
    sqlx::query(
        "UPDATE cost SET reviewed = 0, version = version + 1, updated_at = ? \
         WHERE account_id = ? AND id = ? AND version = ?",
    )
    .bind(now_millis())
    .bind(scope.account_id())
    .bind(&current.id)
    .bind(current.version)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Fills in the unit amounts of `costs` (all of one period).
pub(super) async fn with_unit_amounts(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    period_id: &str,
    mut costs: Vec<Cost>,
) -> DbResult<Vec<Cost>> {
    let rows = sqlx::query(
        "SELECT a.cost_id, a.unit_id, a.amount_cents FROM cost_unit_amount a \
         JOIN cost c ON c.account_id = a.account_id AND c.id = a.cost_id \
         WHERE a.account_id = ? AND c.billing_period_id = ? ORDER BY a.unit_id",
    )
    .bind(scope.account_id())
    .bind(period_id)
    .fetch_all(&mut *conn)
    .await?;
    for row in &rows {
        let cost_id: String = row.try_get("cost_id")?;
        if let Some(cost) = costs.iter_mut().find(|c| c.id == cost_id) {
            cost.unit_amounts.push(UnitAmount {
                unit_id: row.try_get("unit_id")?,
                amount_cents: row.try_get("amount_cents")?,
            });
        }
    }
    Ok(costs)
}

/// Inserts the amount of one unit of a position.
pub(super) async fn insert_unit_amount(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    cost_id: &str,
    unit_id: &str,
    amount_cents: i64,
) -> DbResult<()> {
    let now = now_millis();
    sqlx::query(
        "INSERT INTO cost_unit_amount (account_id, cost_id, unit_id, amount_cents, \
         created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(scope.account_id())
    .bind(cost_id)
    .bind(unit_id)
    .bind(amount_cents)
    .bind(now)
    .bind(now)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
