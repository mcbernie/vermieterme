//! Transactions: rollback keeps the original error, savepoints, SQLite write locks, and
//! the account lock ending sessions atomically.

// Test bodies are plain async fns run by `dual_test!`, so clippy's in-test allowance
// does not reach them.
#![allow(clippy::unwrap_used)]
// Tests build scopes directly (no session layer here); see vermieterme_db::scope.
#![allow(clippy::disallowed_methods)]

#[path = "support/fixtures.rs"]
mod fixtures;

use std::time::Duration;

use fixtures::{HOUR, NOW, setup};
use sqlx::{AnyConnection, Row};
use vermieterme_db::repo::account::{self, AccountPatch};
use vermieterme_db::repo::session;
use vermieterme_db::repo::user::{self, NewUser, Role};
use vermieterme_db::testing::TestDb;
use vermieterme_db::{AdminScope, Db, DbError, SecretToken, dual_test, in_transaction};

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

/// Locking an account ends its members' sessions and moves switched-in admins out, in
/// the same call; renaming leaves sessions alone.
async fn account_lock_ends_sessions(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let admin = AdminScope::system();
    let (scope, member) = setup(&mut c, "m@example.org", Role::Member).await;
    let member_session =
        session::create(&mut c, &member, &SecretToken::generate(), NOW, NOW + HOUR)
            .await
            .unwrap();
    let root = NewUser {
        email: "admin@example.org".into(),
        name: "A".into(),
        role: Role::Admin,
        password_hash: None,
    };
    let root = user::create_for_admin(&mut c, &admin, None, &root)
        .await
        .unwrap();
    let admin_session =
        session::create(&mut c, &root.id, &SecretToken::generate(), NOW, NOW + HOUR)
            .await
            .unwrap();
    session::set_acting_account(&mut c, &admin, &admin_session.id, Some(scope.account_id()))
        .await
        .unwrap();

    let rename = AccountPatch {
        name: Some("Neu".into()),
        locked: None,
    };
    account::update(&mut c, &admin, scope.account_id(), &rename)
        .await
        .unwrap();
    assert_eq!(
        session_ids(&mut c).await.len(),
        2,
        "Umbenennen beendet nichts"
    );

    let lock = AccountPatch {
        name: None,
        locked: Some(true),
    };
    account::update(&mut c, &admin, scope.account_id(), &lock)
        .await
        .unwrap();
    assert_eq!(
        session_ids(&mut c).await,
        std::slice::from_ref(&admin_session.id)
    );
    assert_ne!(member_session.id, admin_session.id);
    let acting: Option<String> =
        sqlx::query("SELECT acting_account_id FROM web_session WHERE id = ?")
            .bind(&admin_session.id)
            .fetch_one(&mut *c)
            .await
            .unwrap()
            .get("acting_account_id");
    assert_eq!(acting, None, "Admin verlässt den gesperrten Kunden");
}
dual_test!(tx_account_lock, account_lock_ends_sessions);

/// Ids of all stored sessions.
async fn session_ids(c: &mut AnyConnection) -> Vec<String> {
    sqlx::query("SELECT id FROM web_session ORDER BY id")
        .fetch_all(&mut *c)
        .await
        .unwrap()
        .iter()
        .map(|row| row.get("id"))
        .collect()
}

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
