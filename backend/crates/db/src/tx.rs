//! Running a group of statements in one transaction.

use sqlx::{AnyConnection, Connection as _};

use crate::dialect::Dialect;
use crate::error::DbResult;

/// Start statement of a top-level SQLite transaction: takes the write lock at once.
const SQLITE_BEGIN_IMMEDIATE: &str = "BEGIN IMMEDIATE";

/// Start statement of a top-level MySQL/MariaDB transaction under `REPEATABLE READ`.
///
/// Every top-level transaction names its isolation explicitly, and in the same statement
/// as its `BEGIN` (one packet, the connection enables multi-statements): `SET
/// TRANSACTION` without a scope applies to the next transaction of the session only, and
/// survives non-transactional statements until one begins. Sent separately, a cancelled
/// future (client gone) or a failed `BEGIN` between the two would leave it on the pooled
/// connection for whatever transaction comes next (#118). Naming the level each time also
/// overrides any such leftover instead of relying on the server default.
const MYSQL_BEGIN_REPEATABLE_READ: &str = "SET TRANSACTION ISOLATION LEVEL REPEATABLE READ; BEGIN";

/// Start statement of a top-level MySQL/MariaDB transaction under `READ COMMITTED`; see
/// [`MYSQL_BEGIN_REPEATABLE_READ`].
const MYSQL_BEGIN_READ_COMMITTED: &str = "SET TRANSACTION ISOLATION LEVEL READ COMMITTED; BEGIN";

/// Runs `body` in a transaction (a savepoint if `conn` is already in one): commits on
/// `Ok`, rolls back explicitly on `Err`. Repositories and the following work packages use
/// this instead of `pool().begin()`.
///
/// - **SQLite:** a top-level transaction starts with `BEGIN IMMEDIATE`, so it holds the
///   write lock from the start. With the default deferred `BEGIN`, a transaction that
///   reads and then writes fails at once with `SQLITE_BUSY` (no busy timeout applies)
///   when another connection wrote in between; with `BEGIN IMMEDIATE` concurrent writers
///   wait for each other (busy timeout) and run one after the other. MySQL/MariaDB begin
///   with `REPEATABLE READ` named in the same statement (row locks), so a level left
///   pending on the pooled connection never carries over to this transaction (#118).
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
/// function too. The level is set in the same statement as `BEGIN`, so a cancelled start
/// leaves nothing pending for the next transaction of the connection (#118). MySQL allows `READ COMMITTED` writes only with row-based or mixed
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
    let begin = match (conn.is_in_transaction(), Dialect::of(conn)?) {
        // Nested: a savepoint, which inherits the isolation of the outer transaction.
        (true, _) => None,
        (false, Dialect::Sqlite) => Some(SQLITE_BEGIN_IMMEDIATE),
        (false, Dialect::MySql) if read_committed => Some(MYSQL_BEGIN_READ_COMMITTED),
        (false, Dialect::MySql) => Some(MYSQL_BEGIN_REPEATABLE_READ),
    };
    let mut tx = match begin {
        Some(statement) => conn.begin_with(statement).await?,
        None => conn.begin().await?,
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
