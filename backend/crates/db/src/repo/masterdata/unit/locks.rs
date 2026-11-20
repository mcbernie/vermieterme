//! Row locks on units (lock order step 2, see `repo::billing`).

use sqlx::{AnyConnection, Row};

use super::{Unit, from_row};
use crate::dialect::Dialect;
use crate::error::{DbError, DbResult};
use crate::scope::AccountScope;

/// Reads a unit and, on MySQL, locks its row until the end of the transaction (SQLite has
/// the write lock from `BEGIN IMMEDIATE`), in one statement. Needed before moving a unit
/// (`detach_from_billings`): without the lock a concurrent insert into
/// `prepayment`/`cost_unit_amount` could commit between check and `UPDATE`. Billing code (P5) locks the same unit row through this
/// function before inserting prepayments or per-unit cost amounts, so both sides
/// serialize on it (decision 37).
pub(crate) async fn lock(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
) -> DbResult<Unit> {
    let sql = Dialect::of(conn)?.pick(
        concat!(
            "SELECT ",
            columns!(),
            " FROM unit WHERE account_id = ? AND id = ?"
        ),
        concat!(
            "SELECT ",
            columns!(),
            " FROM unit WHERE account_id = ? AND id = ? FOR UPDATE"
        ),
    );
    let row = sqlx::query(sql)
        .bind(scope.account_id())
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(DbError::NotFound)?;
    from_row(&row)
}

/// Ids of the units of a property, sorted, locked on MySQL until the end of the
/// transaction (lock order step 2, after the property: see `repo::billing`). Creating a
/// billing period and deleting a property take them, so moving a unit in or out waits.
/// Their callers run under `READ COMMITTED`, so the range read locks only these rows,
/// not the first unit of the next property in the index (#116).
pub(crate) async fn lock_of_property(
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
