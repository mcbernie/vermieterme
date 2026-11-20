//! Running a group of statements in one transaction.

use sqlx::{AnyConnection, Connection as _};

use crate::dialect::Dialect;
use crate::error::DbResult;

/// Start statement of a top-level SQLite transaction: takes the write lock at once.
const SQLITE_BEGIN_IMMEDIATE: &str = "BEGIN IMMEDIATE";

/// Runs `body` in a transaction (a savepoint if `conn` is already in one): commits on
/// `Ok`, rolls back explicitly on `Err`. Repositories and the following work packages use
/// this instead of `pool().begin()`.
///
/// - **SQLite:** a top-level transaction starts with `BEGIN IMMEDIATE`, so it holds the
///   write lock from the start. With the default deferred `BEGIN`, a transaction that
///   reads and then writes fails at once with `SQLITE_BUSY` (no busy timeout applies)
///   when another connection wrote in between; with `BEGIN IMMEDIATE` concurrent writers
///   wait for each other (busy timeout) and run one after the other. MySQL/MariaDB use a
///   normal `BEGIN` (row locks).
/// - **Rollback:** explicit, because a merely dropped transaction leaves the rollback
///   pending until the connection is used again, and a pooled MySQL connection would
///   keep its locks until then. If the rollback itself fails, the error of `body` is
///   returned (it carries the meaning, e.g. a conflict); the rollback error is dropped
///   without logging (this crate has no logger, and the pool tests a returned
///   connection before reusing it).
pub async fn in_transaction<T>(
    conn: &mut AnyConnection,
    body: impl AsyncFnOnce(&mut AnyConnection) -> DbResult<T>,
) -> DbResult<T> {
    let immediate = !conn.is_in_transaction() && Dialect::of(conn)? == Dialect::Sqlite;
    let mut tx = if immediate {
        conn.begin_with(SQLITE_BEGIN_IMMEDIATE).await?
    } else {
        conn.begin().await?
    };
    match body(&mut tx).await {
        Ok(value) => {
            tx.commit().await?;
            Ok(value)
        }
        Err(error) => {
            // Deliberately ignored: the error of `body` is the one the caller needs.
            let _ = tx.rollback().await;
            Err(error)
        }
    }
}
