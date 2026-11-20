//! Running a group of statements in one transaction.

use sqlx::{AnyConnection, Connection as _};

use crate::error::DbResult;

/// Runs `body` in a transaction (a savepoint if `conn` is already in one): commits on
/// `Ok`, rolls back explicitly on `Err`. The explicit rollback matters: a merely dropped
/// transaction leaves the rollback pending until the connection is used again, and a
/// pooled MySQL connection would keep its locks until then.
pub(crate) async fn in_transaction<T>(
    conn: &mut AnyConnection,
    body: impl AsyncFnOnce(&mut AnyConnection) -> DbResult<T>,
) -> DbResult<T> {
    let mut tx = conn.begin().await?;
    match body(&mut tx).await {
        Ok(value) => {
            tx.commit().await?;
            Ok(value)
        }
        Err(error) => {
            tx.rollback().await?;
            Err(error)
        }
    }
}
