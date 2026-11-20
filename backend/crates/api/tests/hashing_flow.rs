//! Password hashing under load: requests waiting for a hash permit or computing hold no
//! pool connection, so a login flood does not block the rest of the application (#102);
//! cancelled checks still count against the limit (#104); over-long passwords cost no
//! argon2 work.

// Same convention as the other test binaries: helpers unwrap.
#![allow(clippy::unwrap_used)]

mod support;

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::json;
use support::{PASSWORD, TestApp, config, seed_account, seed_user};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use vermieterme_api::error::validate::PASSWORD_MAX;
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

/// Sends a failing login for `email` and closes the connection once its password check
/// has started (the client gives up while argon2 runs).
async fn cancelled_login(app: &TestApp, email: &str) {
    let body = json!({"email": email, "password": "falsch-falsch"}).to_string();
    let request = format!(
        "POST /api/auth/login HTTP/1.0\r\nHost: {}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nX-CSRF-Guard: 1\r\n\r\n{body}",
        app.addr,
        body.len()
    );
    let before = app.hasher.computed();
    let mut stream = TcpStream::connect(app.addr).await.unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    while app.hasher.computed() == before {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    drop(stream);
}

/// Logins cancelled after their password check started count as failures: after 20 of
/// them the address is locked (`429` with the lock's `Retry-After`, not the short "busy"
/// answer of running attempts).
#[tokio::test]
async fn cancelled_logins_lock_the_address() {
    let test_db = TestDb::sqlite().await;
    let app = TestApp::start(test_db.db()).await;
    for i in 0..20 {
        cancelled_login(&app, &format!("x{i}@example.org")).await;
    }
    let probe = json!({"email": "probe@example.org", "password": "falsch-falsch"});
    for _ in 0..100 {
        let resp = app
            .call("POST", "/api/auth/login", None, probe.clone())
            .await;
        assert_eq!(resp.status, 429, "{:?}", resp.body);
        let retry: u64 = resp.header("retry-after").unwrap().parse().unwrap();
        if retry > 60 {
            return;
        }
        // Cancelled requests not yet dropped by the server still count as running.
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("address not locked");
}

/// `PUT /api/profile` refuses an over-long current password like a wrong one, without
/// argon2 work.
#[tokio::test]
async fn overlong_current_password_costs_no_hash() {
    let test_db = TestDb::sqlite().await;
    let db = test_db.db();
    let app = TestApp::start(db).await;
    let account = seed_account(db, "Kunde").await;
    seed_user(db, Some(&account.id), "p@example.org", Role::Member).await;
    let cookie = app.login("p@example.org", PASSWORD).await;
    let change =
        |current: String| json!({"currentPassword": current, "newPassword": "neues-passwort-1"});

    let wrong = app
        .call(
            "PUT",
            "/api/profile",
            Some(&cookie),
            change("falsch-falsch".into()),
        )
        .await;
    let before = app.hasher.computed();
    let long = change("x".repeat(PASSWORD_MAX + 1));
    let resp = app.call("PUT", "/api/profile", Some(&cookie), long).await;
    assert_eq!(app.hasher.computed(), before);
    assert_eq!((resp.status, &resp.body), (wrong.status, &wrong.body));
    assert_eq!(resp.body["error"], "Aktuelles Passwort ist falsch");
}
