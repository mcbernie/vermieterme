//! Running a group of statements in one transaction.

use sqlx::{AnyConnection, Connection as _};

use crate::dialect::Dialect;
use crate::error::DbResult;

/// Start statement of a top-level SQLite transaction: takes the write lock at once.
const SQLITE_BEGIN_IMMEDIATE: &str = "BEGIN IMMEDIATE";

/// Sets the isolation of the next MySQL/MariaDB transaction only (not the session), so a
/// pooled connection returns to the server default afterwards.
const MYSQL_NEXT_READ_COMMITTED: &str = "SET TRANSACTION ISOLATION LEVEL READ COMMITTED";

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
    run(conn, false, body).await
}

/// Like [`in_transaction`], but a top-level MySQL/MariaDB transaction runs with
/// `READ COMMITTED` instead of the server default `REPEATABLE READ` (SQLite: unchanged,
/// `BEGIN IMMEDIATE` serializes anyway).
///
/// For changes that serialize on explicit row locks (the lock order of
/// `repo::billing`, #116): under `READ COMMITTED` InnoDB takes no gap or next-key locks
/// for searches and scans, so a locking range read or a cascade never reaches into the
/// index range of a neighbouring property or account, and every plain read sees the
/// latest commit (no snapshot older than the locks). Changes that rely on gap locks to
/// keep rows from appearing (e.g. the last-admin rule) keep using [`in_transaction`].
///
/// The isolation can only be chosen before `BEGIN`: inside a transaction this opens a
/// savepoint of the outer one and inherits its isolation, so a caller that wraps these
/// changes (the API with its audit entry) must start the outer transaction with this
/// function too. MySQL allows `READ COMMITTED` writes only with row-based or mixed
/// binary logging (`binlog_format`, default `ROW`).
pub async fn in_transaction_read_committed<T>(
    conn: &mut AnyConnection,
    body: impl AsyncFnOnce(&mut AnyConnection) -> DbResult<T>,
) -> DbResult<T> {
    run(conn, true, body).await
}

/// Begins a transaction (or savepoint), runs `body`, commits or rolls back; see
/// [`in_transaction`]. `read_committed` applies to a top-level MySQL transaction only.
async fn run<T>(
    conn: &mut AnyConnection,
    read_committed: bool,
    body: impl AsyncFnOnce(&mut AnyConnection) -> DbResult<T>,
) -> DbResult<T> {
    let top_level = !conn.is_in_transaction();
    let dialect = Dialect::of(conn)?;
    if top_level && read_committed && dialect == Dialect::MySql {
        // Text protocol: applies to the `BEGIN` right below and to nothing after it.
        sqlx::raw_sql(MYSQL_NEXT_READ_COMMITTED)
            .execute(&mut *conn)
            .await?;
    }
    let mut tx = if top_level && dialect == Dialect::Sqlite {
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
