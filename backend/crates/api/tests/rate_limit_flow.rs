//! Rate limits over the real router: parallel attempts, client address behind proxies;
//! plus the error shape of unknown paths and methods.
//! The limiter lives in memory, so SQLite is enough here.

// Same convention as the other test binaries: helpers unwrap.
#![allow(clippy::unwrap_used)]

mod support;

use std::sync::Arc;

use serde_json::json;
use support::{PASSWORD, TestApp, seed_account, seed_user};
use vermieterme_api::error::validate::PASSWORD_MAX;
use vermieterme_db::repo::user::Role;
use vermieterme_db::testing::TestDb;

/// Parallel logins for one e-mail: at most the limit (5) reaches the password check,
/// the others are answered 429 at once.
#[tokio::test]
async fn parallel_logins_respect_the_limit() {
    let test_db = TestDb::sqlite().await;
    let db = test_db.db();
    let app = Arc::new(TestApp::start(db).await);
    let account = seed_account(db, "Kunde").await;
    seed_user(db, Some(&account.id), "p@example.org", Role::Member).await;
    let wrong = json!({"email": "p@example.org", "password": "falsch-falsch"});
    let tasks: Vec<_> = (0..20)
        .map(|_| {
            let app = app.clone();
            let body = wrong.clone();
            tokio::spawn(async move { app.call("POST", "/api/auth/login", None, body).await })
        })
        .collect();
    let mut statuses = Vec::new();
    for task in tasks {
        statuses.push(task.await.unwrap().status);
    }
    let unauthorized = statuses.iter().filter(|s| **s == 401).count();
    let limited = statuses.iter().filter(|s| **s == 429).count();
    assert!(unauthorized <= 5, "{statuses:?}");
    assert_eq!(unauthorized + limited, 20, "{statuses:?}");
}

/// An over-long e-mail is refused with the generic answer and counts against the address.
#[tokio::test]
async fn huge_email_is_refused_generically() {
    let test_db = TestDb::sqlite().await;
    let app = TestApp::start(test_db.db()).await;
    let body = json!({"email": "x".repeat(1_000_000), "password": "falsch-falsch"});
    let resp = app.call("POST", "/api/auth/login", None, body).await;
    assert_eq!(resp.status, 401);
    assert_eq!(resp.body["error"], "Anmeldung fehlgeschlagen");
}

/// An over-long password counts only against the address: it creates no e-mail key and
/// so cannot lock the account, and it costs no argon2 work (#101).
#[tokio::test]
async fn overlong_password_does_not_count_against_the_email() {
    let test_db = TestDb::sqlite().await;
    let db = test_db.db();
    let app = TestApp::start(db).await;
    let account = seed_account(db, "Kunde").await;
    seed_user(db, Some(&account.id), "o@example.org", Role::Member).await;
    let long = json!({"email": "o@example.org", "password": "x".repeat(PASSWORD_MAX + 1)});
    for _ in 0..6 {
        let resp = app
            .call("POST", "/api/auth/login", None, long.clone())
            .await;
        assert_eq!(resp.status, 401);
        assert_eq!(resp.body["error"], "Anmeldung fehlgeschlagen");
    }
    assert_eq!(app.hasher.computed(), 0);
    app.login("o@example.org", PASSWORD).await;
}

/// Starts `n` failing logins at once, each for its own e-mail (so only the address limit
/// applies), optionally with an `X-Forwarded-For` header; returns their statuses.
async fn fail_logins(app: &Arc<TestApp>, n: usize, forwarded: Option<&str>) -> Vec<u16> {
    let tasks: Vec<_> = (0..n)
        .map(|i| {
            let app = app.clone();
            let forwarded = forwarded.map(str::to_owned);
            tokio::spawn(async move {
                let body =
                    json!({"email": format!("x{i}@example.org"), "password": "falsch-falsch"});
                let extra: Vec<(&str, &str)> = forwarded
                    .as_deref()
                    .map(|f| vec![("x-forwarded-for", f)])
                    .unwrap_or_default();
                app.send("POST", "/api/auth/login", None, Some(body), &extra)
                    .await
                    .status
            })
        })
        .collect();
    let mut statuses = Vec::new();
    for task in tasks {
        statuses.push(task.await.unwrap());
    }
    statuses
}

/// Without `TRUSTED_PROXIES` the header is ignored: all clients share the peer address,
/// so the 21st failure is refused whatever the header says.
#[tokio::test]
async fn forwarded_for_is_ignored_without_trusted_proxies() {
    let test_db = TestDb::sqlite().await;
    let app = Arc::new(TestApp::start(test_db.db()).await);
    let statuses = fail_logins(&app, 20, Some("198.51.100.1")).await;
    assert!(statuses.iter().all(|s| *s == 401), "{statuses:?}");
    let next = fail_logins(&app, 1, Some("198.51.100.2")).await;
    assert_eq!(next, [429]);
}

/// Behind a trusted proxy each forwarded client has its own budget.
#[tokio::test]
async fn forwarded_for_counts_per_client_behind_trusted_proxy() {
    let test_db = TestDb::sqlite().await;
    let config = support::config(&[("TRUSTED_PROXIES", "127.0.0.1")]);
    let app = Arc::new(TestApp::start_with(test_db.db(), config).await);
    let statuses = fail_logins(&app, 20, Some("198.51.100.1")).await;
    assert!(statuses.iter().all(|s| *s == 401), "{statuses:?}");
    assert_eq!(fail_logins(&app, 1, Some("198.51.100.1")).await, [429]);
    assert_eq!(fail_logins(&app, 1, Some("198.51.100.2")).await, [401]);
    // Same /64 shares the budget, another /64 does not.
    let v6 = fail_logins(&app, 20, Some("2001:db8:1:2::1")).await;
    assert!(v6.iter().all(|s| *s == 401), "{v6:?}");
    assert_eq!(
        fail_logins(&app, 1, Some("2001:db8:1:2::ffff")).await,
        [429]
    );
    assert_eq!(fail_logins(&app, 1, Some("2001:db8:1:3::1")).await, [401]);
}

/// Unknown API paths and wrong methods answer in the contract's error shape.
#[tokio::test]
async fn unknown_paths_and_methods_use_the_error_shape() {
    let test_db = TestDb::sqlite().await;
    let app = TestApp::start(test_db.db()).await;
    let resp = app.get("/api/gibt-es-nicht", None).await;
    assert_eq!(resp.status, 404);
    assert_eq!(
        resp.body,
        json!({"error": "Nicht gefunden", "code": "not_found"})
    );
    let resp = app.get("/api/auth/login", None).await;
    assert_eq!(resp.status, 405);
    assert_eq!(resp.body["code"], "method_not_allowed");
    let resp = app
        .call("DELETE", "/api/health", None, serde_json::Value::Null)
        .await;
    assert_eq!(resp.status, 405);
    assert_eq!(resp.header("cache-control"), Some("no-store"));
}
