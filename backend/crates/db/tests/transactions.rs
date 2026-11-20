//! Transactions: rollback keeps the original error, savepoints, SQLite write locks.

// Test bodies are plain async fns run by `dual_test!`, so clippy's in-test allowance
// does not reach them.
#![allow(clippy::unwrap_used)]
// Tests build scopes directly (no session layer here); see vermieterme_db::scope.
#![allow(clippy::disallowed_methods)]

use std::time::Duration;

use sqlx::{AnyConnection, Row};
use vermieterme_db::repo::account;
use vermieterme_db::testing::TestDb;
use vermieterme_db::{AdminScope, Db, DbError, dual_test, in_transaction};

/// Number of accounts (the rollback tests write accounts).
async fn account_count(c: &mut AnyConnection) -> i64 {
    sqlx::query("SELECT COUNT(*) AS n FROM account")
        .fetch_one(&mut *c)
        .await
        .unwrap()
        .get("n")
}

/// An error of the body is returned unchanged and its writes are rolled back; an error
/// inside a nested call rolls back only that savepoint.
async fn rollback_and_savepoint(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let admin = AdminScope::system();

    let result: Result<(), DbError> = in_transaction(&mut c, async |tx| {
        account::create(tx, &admin, "verworfen").await?;
        Err(DbError::Conflict)
    })
    .await;
    assert!(matches!(result, Err(DbError::Conflict)), "{result:?}");
    assert_eq!(account_count(&mut c).await, 0, "zurückgerollt");

    in_transaction(&mut c, async |tx| {
        account::create(tx, &admin, "bleibt").await?;
        let inner: Result<(), DbError> = in_transaction(tx, async |sp| {
            account::create(sp, &admin, "Sicherungspunkt").await?;
            Err(DbError::NotFound)
        })
        .await;
        assert!(matches!(inner, Err(DbError::NotFound)));
        Ok(())
    })
    .await
    .unwrap();
    let names: Vec<String> = account::list(&mut c, &admin)
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.account.name)
        .collect();
    assert_eq!(names, ["bleibt"]);
}
dual_test!(tx_rollback, rollback_and_savepoint);

/// Reads the counter, waits so the other transaction overlaps, then writes it + 1.
async fn increment(db: &Db) -> Result<(), DbError> {
    let mut c = db.pool().acquire().await.unwrap();
    in_transaction(&mut c, async |tx| {
        let n: i64 = sqlx::query("SELECT n FROM tx_counter WHERE id = 1")
            .fetch_one(&mut *tx)
            .await?
            .get("n");
        tokio::time::sleep(Duration::from_millis(50)).await;
        sqlx::query("UPDATE tx_counter SET n = ? WHERE id = 1")
            .bind(n + 1)
            .execute(&mut *tx)
            .await?;
        Ok(())
    })
    .await
}

/// Two concurrent "read, then write" transactions under SQLite both succeed one after
/// the other (`BEGIN IMMEDIATE`): no `SQLITE_BUSY`, no lost update.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_read_then_write_transactions_serialise() {
    let test_db = TestDb::sqlite().await;
    let db = test_db.db();
    sqlx::query("CREATE TABLE tx_counter (id INTEGER PRIMARY KEY, n BIGINT NOT NULL)")
        .execute(db.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO tx_counter (id, n) VALUES (1, 0)")
        .execute(db.pool())
        .await
        .unwrap();

    let (first, second) = tokio::join!(increment(db), increment(db));
    assert!(first.is_ok(), "{first:?}");
    assert!(second.is_ok(), "{second:?}");
    let n: i64 = sqlx::query("SELECT n FROM tx_counter WHERE id = 1")
        .fetch_one(db.pool())
        .await
        .unwrap()
        .get("n");
    assert_eq!(n, 2, "beide Erhöhungen nacheinander");
}
