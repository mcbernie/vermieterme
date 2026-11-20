//! Rent and prepayment changes (Mietänderungen) of a unit.

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};

use super::unit;
use crate::error::{DbError, DbResult};
use crate::row::{new_id, now_millis};
use crate::scope::AccountScope;
use crate::tx::in_transaction;

/// Allowed values of [`RentChangeData::change_type`].
pub const TYPES: [&str; 2] = ["rent", "prepayment"];

/// The editable values of a rent change (validated and trimmed by the caller).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RentChangeData {
    /// The unit.
    pub unit_id: String,
    /// One of [`TYPES`].
    pub change_type: String,
    /// New monthly amount in cents.
    pub amount_cents: i64,
    /// First day the amount applies (`YYYY-MM-DD`).
    pub effective_date: String,
    /// Optional reason.
    pub reason: Option<String>,
}

/// A stored rent change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RentChange {
    /// Primary key.
    pub id: String,
    /// The values.
    pub data: RentChangeData,
    /// Creation time (Unix ms).
    pub created_at: i64,
    /// Last change (Unix ms).
    pub updated_at: i64,
}

/// Column list read by [`from_row`].
macro_rules! columns {
    () => {
        "id, unit_id, change_type, amount_cents, effective_date, reason, created_at, updated_at"
    };
}

/// Maps a row selected with `columns!()`.
fn from_row(row: &AnyRow) -> DbResult<RentChange> {
    Ok(RentChange {
        id: row.try_get("id")?,
        data: RentChangeData {
            unit_id: row.try_get("unit_id")?,
            change_type: row.try_get("change_type")?,
            amount_cents: row.try_get("amount_cents")?,
            effective_date: row.try_get("effective_date")?,
            reason: row.try_get("reason")?,
        },
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

/// Lists the changes of the account (optionally of one unit), latest effective date first.
pub async fn list(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    unit_id: Option<&str>,
) -> DbResult<Vec<RentChange>> {
    let sql = match unit_id {
        Some(_) => concat!(
            "SELECT ",
            columns!(),
            " FROM rent_change WHERE account_id = ? AND unit_id = ? \
             ORDER BY effective_date DESC, created_at DESC, id DESC"
        ),
        None => concat!(
            "SELECT ",
            columns!(),
            " FROM rent_change WHERE account_id = ? \
             ORDER BY effective_date DESC, created_at DESC, id DESC"
        ),
    };
    let mut query = sqlx::query(sql).bind(scope.account_id());
    if let Some(unit_id) = unit_id {
        query = query.bind(unit_id);
    }
    let rows = query.fetch_all(&mut *conn).await?;
    rows.iter().map(from_row).collect()
}

/// Reads one change of the account.
pub async fn get(conn: &mut AnyConnection, scope: &AccountScope, id: &str) -> DbResult<RentChange> {
    let row = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM rent_change WHERE account_id = ? AND id = ?"
    ))
    .bind(scope.account_id())
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(DbError::NotFound)?;
    from_row(&row)
}

/// Creates a change; a foreign or unknown unit is [`DbError::NotFound`].
pub async fn create(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    data: &RentChangeData,
) -> DbResult<RentChange> {
    in_transaction(conn, async |tx| {
        unit::ensure(tx, scope, &data.unit_id).await?;
        let id = new_id();
        let now = now_millis();
        sqlx::query(
            "INSERT INTO rent_change (id, account_id, unit_id, change_type, amount_cents, \
             effective_date, reason, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(scope.account_id())
        .bind(&data.unit_id)
        .bind(&data.change_type)
        .bind(data.amount_cents)
        .bind(&data.effective_date)
        .bind(data.reason.as_deref())
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        get(tx, scope, &id).await
    })
    .await
}

/// Replaces the values of a change (the unit may change).
pub async fn update(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
    data: &RentChangeData,
) -> DbResult<RentChange> {
    in_transaction(conn, async |tx| {
        get(tx, scope, id).await?;
        unit::ensure(tx, scope, &data.unit_id).await?;
        sqlx::query(
            "UPDATE rent_change SET unit_id = ?, change_type = ?, amount_cents = ?, \
             effective_date = ?, reason = ?, updated_at = ? WHERE account_id = ? AND id = ?",
        )
        .bind(&data.unit_id)
        .bind(&data.change_type)
        .bind(data.amount_cents)
        .bind(&data.effective_date)
        .bind(data.reason.as_deref())
        .bind(now_millis())
        .bind(scope.account_id())
        .bind(id)
        .execute(&mut *tx)
        .await?;
        get(tx, scope, id).await
    })
    .await
}

/// Deletes a change.
pub async fn delete(conn: &mut AnyConnection, scope: &AccountScope, id: &str) -> DbResult<()> {
    let result = sqlx::query("DELETE FROM rent_change WHERE account_id = ? AND id = ?")
        .bind(scope.account_id())
        .bind(id)
        .execute(&mut *conn)
        .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::NotFound);
    }
    Ok(())
}
