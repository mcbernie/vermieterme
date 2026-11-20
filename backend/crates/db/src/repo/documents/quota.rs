//! Storage quota of an account (decision 44, #124): the sum of `size_bytes` of its
//! documents may not exceed a limit.
//!
//! Two uploads of the same account may lock different properties and would both pass a
//! plain sum check before either commits. So the check locks the **account** row first
//! (MySQL `FOR UPDATE`; SQLite serialises writers from `BEGIN IMMEDIATE`): the second
//! upload waits for the first commit and, under `READ COMMITTED`, then sees its row.
//!
//! The account row is the **last** lock an upload takes (after its property, unit and
//! tenant locks, lock order of `repo::billing`); nothing locks the account row and then a
//! property, so the order stays acyclic. Other writers only take shared locks on the
//! account row (foreign-key checks of their inserts) after their own locks.

use sqlx::{AnyConnection, Row};

use crate::dialect::Dialect;
use crate::error::{DbError, DbResult};
use crate::scope::AccountScope;

/// The document would take the account beyond its storage quota.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuotaExceeded;

/// Locks the account row and checks that `add` more bytes fit into `quota` (bytes).
/// Call inside the transaction of the insert, after the other locks.
pub(super) async fn check(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    add: i64,
    quota: i64,
) -> DbResult<Result<(), QuotaExceeded>> {
    lock_account(conn, scope).await?;
    let used = used_bytes(conn, scope).await?;
    if used.saturating_add(add) > quota {
        return Ok(Err(QuotaExceeded));
    }
    Ok(Ok(()))
}

/// Locks the row of the scope's account until the end of the transaction.
async fn lock_account(conn: &mut AnyConnection, scope: &AccountScope) -> DbResult<()> {
    let sql = Dialect::of(conn)?.pick(
        "SELECT id FROM account WHERE id = ?",
        "SELECT id FROM account WHERE id = ? FOR UPDATE",
    );
    sqlx::query(sql)
        .bind(scope.account_id())
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(DbError::NotFound)?;
    Ok(())
}

/// Bytes of all documents of the account (plain read; under the account lock it sees
/// every committed upload of the account).
pub async fn used_bytes(conn: &mut AnyConnection, scope: &AccountScope) -> DbResult<i64> {
    let row = sqlx::query(
        "SELECT CAST(COALESCE(SUM(size_bytes), 0) AS SIGNED) AS used \
         FROM document WHERE account_id = ?",
    )
    .bind(scope.account_id())
    .fetch_one(&mut *conn)
    .await?;
    Ok(row.try_get("used")?)
}
