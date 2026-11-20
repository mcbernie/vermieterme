//! Password hashing under load: requests waiting for a hash permit or computing hold no
//! pool connection, so a login flood does not block the rest of the application (#102).

// Same convention as the other test binaries: helpers unwrap.
#![allow(clippy::unwrap_used)]

mod support;

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::json;
use support::{PASSWORD, TestApp, config, seed_account, seed_user};
use vermieterme_db::repo::user::Role;
use vermieterme_db::testing::TestDb;

/// More logins than the SQLite pool has connections wait for the only hash permit; the
/// health check and a signed-in request still answer at once.
#[tokio::test]
async fn waiting_logins_do_not_block_the_pool() {
    let test_db = TestDb::sqlite().await;
    let db = test_db.db();
    let cfg = config(&[("PASSWORD_HASH_CONCURRENCY", "1")]);
    let app = Arc::new(TestApp::start_with(db, cfg).await);
    let account = seed_account(db, "Kunde").await;
    seed_user(db, Some(&account.id), "h@example.org", Role::Member).await;
    let cookie = app.login("h@example.org", PASSWORD).await;

    let held = app.hasher.permit().await.unwrap();
    let logins: Vec<_> = (0..8)
        .map(|i| {
            let app = app.clone();
            let body = json!({"email": format!("x{i}@example.org"), "password": "falsch-falsch"});
            tokio::spawn(async move { app.call("POST", "/api/auth/login", None, body).await })
        })
        .collect();
    // Let the logins read their user and queue for the permit.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let started = Instant::now();
    assert_eq!(app.get("/api/health", None).await.status, 200);
    assert_eq!(
        app.get("/api/auth/session", Some(&cookie)).await.status,
        200
    );
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );

    drop(held);
    for login in logins {
        let status = login.await.unwrap().status;
        assert!(status == 401 || status == 429, "{status}");
    }
}
