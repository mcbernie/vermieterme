//! Cost categories (Kostenarten) of an account.

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};

use super::{Checked, Rejection};
use crate::dialect::Dialect;
use crate::error::{DbError, DbResult};
use crate::row::{new_id, now_millis};
use crate::scope::AccountScope;
use crate::tx::in_transaction;

/// A cost category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostCategory {
    /// Primary key.
    pub id: String,
    /// Name.
    pub name: String,
    /// Distribution key code (`mea`, `per_notice`, `per_attachment`, `per_unit`).
    pub distribution_key: String,
    /// Position in lists, ascending.
    pub sort_order: i64,
    /// Creation time (Unix ms).
    pub created_at: i64,
    /// Last change (Unix ms).
    pub updated_at: i64,
}

/// Values of create and update (validated and trimmed by the caller).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostCategoryInput {
    /// Name.
    pub name: String,
    /// Distribution key code, one of the codes of `DistributionKey`.
    pub distribution_key: String,
    /// Position in lists.
    pub sort_order: i64,
}

/// Column list read by [`from_row`].
macro_rules! columns {
    () => {
        "id, name, distribution_key, sort_order, created_at, updated_at"
    };
}

/// Maps a row selected with `columns!()`.
fn from_row(row: &AnyRow) -> DbResult<CostCategory> {
    Ok(CostCategory {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        distribution_key: row.try_get("distribution_key")?,
        sort_order: row.try_get("sort_order")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

/// Lists the categories of the account by `sort_order`, then name.
pub async fn list(conn: &mut AnyConnection, scope: &AccountScope) -> DbResult<Vec<CostCategory>> {
    let rows = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM cost_category WHERE account_id = ? ORDER BY sort_order, name, id"
    ))
    .bind(scope.account_id())
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(from_row).collect()
}

/// Reads one category of the account.
pub async fn get(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
) -> DbResult<CostCategory> {
    let row = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM cost_category WHERE account_id = ? AND id = ?"
    ))
    .bind(scope.account_id())
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(DbError::NotFound)?;
    from_row(&row)
}

/// Creates a category.
pub async fn create(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    input: &CostCategoryInput,
) -> DbResult<CostCategory> {
    let id = new_id();
    let now = now_millis();
    sqlx::query(
        "INSERT INTO cost_category (id, account_id, name, distribution_key, sort_order, \
         created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(scope.account_id())
    .bind(&input.name)
    .bind(&input.distribution_key)
    .bind(input.sort_order)
    .bind(now)
    .bind(now)
    .execute(&mut *conn)
    .await?;
    get(conn, scope, &id).await
}

/// Replaces the values of a category.
pub async fn update(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
    input: &CostCategoryInput,
) -> DbResult<CostCategory> {
    let result = sqlx::query(
        "UPDATE cost_category SET name = ?, distribution_key = ?, sort_order = ?, updated_at = ? \
         WHERE account_id = ? AND id = ?",
    )
    .bind(&input.name)
    .bind(&input.distribution_key)
    .bind(input.sort_order)
    .bind(now_millis())
    .bind(scope.account_id())
    .bind(id)
    .execute(&mut *conn)
    .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::NotFound);
    }
    get(conn, scope, id).await
}

/// Deletes a category. A category used by costs of a billing period is refused with
/// [`Rejection::CategoryInUse`] instead of cascading (the schema has no cascade here).
/// The category row is locked first, so a cost saved at the same time (it locks the
/// category too) is either seen by the check (409) or finds the category gone (404),
/// never a foreign-key error.
pub async fn delete(conn: &mut AnyConnection, scope: &AccountScope, id: &str) -> Checked<()> {
    in_transaction(conn, async |tx| {
        lock(tx, scope, id).await?;
        let used = sqlx::query(
            "SELECT 1 AS used FROM cost WHERE account_id = ? AND cost_category_id = ? LIMIT 1",
        )
        .bind(scope.account_id())
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        if used.is_some() {
            return Ok(Err(Rejection::CategoryInUse));
        }
        sqlx::query("DELETE FROM cost_category WHERE account_id = ? AND id = ?")
            .bind(scope.account_id())
            .bind(id)
            .execute(&mut *tx)
            .await?;
        Ok(Ok(()))
    })
    .await
}

/// Reads a category and, on MySQL, locks its row until the end of the transaction (SQLite
/// holds the write lock from `BEGIN IMMEDIATE`). Deleting a category and saving a cost of
/// it (billing, P5) both start here, so they never run side by side.
pub(crate) async fn lock(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
) -> DbResult<CostCategory> {
    let sql = Dialect::of(conn)?.pick(
        concat!(
            "SELECT ",
            columns!(),
            " FROM cost_category WHERE account_id = ? AND id = ?"
        ),
        concat!(
            "SELECT ",
            columns!(),
            " FROM cost_category WHERE account_id = ? AND id = ? FOR UPDATE"
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
