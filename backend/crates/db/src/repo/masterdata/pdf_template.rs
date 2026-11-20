//! The PDF template of an account (one row per account); `config` is JSON text that the
//! API layer has validated.

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};

use crate::error::{DbError, DbResult};
use crate::row::{new_id, now_millis};
use crate::scope::AccountScope;
use crate::tx::in_transaction;

/// Longest allowed `config` text (column limit).
pub const CONFIG_MAX: usize = 16_000;

/// A stored template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfTemplate {
    /// Primary key.
    pub id: String,
    /// Name.
    pub name: String,
    /// Configuration as JSON text.
    pub config: String,
    /// Last change (Unix ms).
    pub updated_at: i64,
}

/// Maps a row of `id, name, config, updated_at`.
fn from_row(row: &AnyRow) -> DbResult<PdfTemplate> {
    Ok(PdfTemplate {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        config: row.try_get("config")?,
        updated_at: row.try_get("updated_at")?,
    })
}

/// Reads the template of the account, `None` while nothing is stored.
pub async fn get(conn: &mut AnyConnection, scope: &AccountScope) -> DbResult<Option<PdfTemplate>> {
    let row =
        sqlx::query("SELECT id, name, config, updated_at FROM pdf_template WHERE account_id = ?")
            .bind(scope.account_id())
            .fetch_optional(&mut *conn)
            .await?;
    row.as_ref().map(from_row).transpose()
}

/// Stores the template. `name` `None` keeps the stored name ("Standard" for a new one).
pub async fn save(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    config: &str,
    name: Option<&str>,
) -> DbResult<PdfTemplate> {
    in_transaction(conn, async |tx| {
        let now = now_millis();
        let result = sqlx::query(
            "UPDATE pdf_template SET config = ?, name = COALESCE(?, name), updated_at = ? \
             WHERE account_id = ?",
        )
        .bind(config)
        .bind(name)
        .bind(now)
        .bind(scope.account_id())
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            sqlx::query(
                "INSERT INTO pdf_template (id, account_id, name, config, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(new_id())
            .bind(scope.account_id())
            .bind(name.unwrap_or("Standard"))
            .bind(config)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }
        get(tx, scope).await?.ok_or(DbError::NotFound)
    })
    .await
}
