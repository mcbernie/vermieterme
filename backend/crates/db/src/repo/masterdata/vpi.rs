//! Consumer price index values (global public statistics, not per account). Reading
//! needs no scope; writing needs the admin scope because every account sees the values.

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};

use crate::dialect::Dialect;
use crate::error::{DbError, DbResult};
use crate::row::{new_id, now_millis};
use crate::scope::AdminScope;
use crate::tx::in_transaction;

/// A stored index value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VpiRow {
    /// Year of the observation.
    pub year: i64,
    /// Month 1-12.
    pub month: i64,
    /// Value in tenths (`1196` = 119.6).
    pub value_tenths: i64,
    /// Base year of the series.
    pub base_year: i64,
}

/// A stored index value with its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VpiRecord {
    /// Primary key.
    pub id: String,
    /// The value.
    pub entry: VpiRow,
}

/// Maps a row of `id, year, month, value_tenths, base_year`.
fn from_row(row: &AnyRow) -> DbResult<VpiRecord> {
    Ok(VpiRecord {
        id: row.try_get("id")?,
        entry: VpiRow {
            year: row.try_get("year")?,
            month: row.try_get("month")?,
            value_tenths: row.try_get("value_tenths")?,
            base_year: row.try_get("base_year")?,
        },
    })
}

/// All values, latest base year, year and month first. Public data: every signed-in
/// account may read it, so no scope is needed.
pub async fn list(conn: &mut AnyConnection) -> DbResult<Vec<VpiRecord>> {
    let rows = sqlx::query(
        "SELECT id, year, month, value_tenths, base_year FROM vpi_entry \
         ORDER BY base_year DESC, year DESC, month DESC",
    )
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(from_row).collect()
}

/// Insert of a new value (the dialect appends its conflict clause).
macro_rules! insert {
    () => {
        "INSERT INTO vpi_entry (id, year, month, value_tenths, base_year, \
     created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
    };
}

/// Stores a value: updates the entry of `year + month + base_year` or creates it.
pub async fn upsert(
    conn: &mut AnyConnection,
    _admin: &AdminScope,
    entry: &VpiRow,
) -> DbResult<VpiRecord> {
    in_transaction(conn, async |tx| {
        let now = now_millis();
        // Create-if-absent first, then update (see `landlord::save`).
        sqlx::query(Dialect::of(tx)?.pick(
            concat!(insert!(), " ON CONFLICT DO NOTHING"),
            concat!(insert!(), " ON DUPLICATE KEY UPDATE id = id"),
        ))
        .bind(new_id())
        .bind(entry.year)
        .bind(entry.month)
        .bind(entry.value_tenths)
        .bind(entry.base_year)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE vpi_entry SET value_tenths = ?, updated_at = ? \
             WHERE year = ? AND month = ? AND base_year = ?",
        )
        .bind(entry.value_tenths)
        .bind(now)
        .bind(entry.year)
        .bind(entry.month)
        .bind(entry.base_year)
        .execute(&mut *tx)
        .await?;
        let row = sqlx::query(
            "SELECT id, year, month, value_tenths, base_year FROM vpi_entry \
             WHERE year = ? AND month = ? AND base_year = ?",
        )
        .bind(entry.year)
        .bind(entry.month)
        .bind(entry.base_year)
        .fetch_one(&mut *tx)
        .await?;
        from_row(&row)
    })
    .await
}

/// Deletes one value.
pub async fn delete(conn: &mut AnyConnection, _admin: &AdminScope, id: &str) -> DbResult<()> {
    let result = sqlx::query("DELETE FROM vpi_entry WHERE id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::NotFound);
    }
    Ok(())
}
