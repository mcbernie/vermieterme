//! Sign-in, session, logout, legacy hashes, password change, CSRF and rate limits over
//! the real router. Flows that touch SQL run against SQLite and (with `TEST_MYSQL_URL`)
//! MariaDB.

// Test bodies and helpers are plain async fns run by `dual_test!`, so clippy's in-test
// allowance for `unwrap` does not cover them (same convention as the db crate's tests).
#![allow(clippy::unwrap_used)]

mod support;

use serde_json::{Value, json};
use support::{PASSWORD, TestApp, seed_account, seed_user, seed_user_with_hash, stored_hash};
use vermieterme_db::repo::user::Role;
use vermieterme_db::{Db, PasswordHash, dual_test};

/// Login sets a hardened cookie, the session document names user and account, logout
/// ends the session.
async fn login_session_logout(db: &Db) {
    let app = TestApp::start(db).await;
    let account = seed_account(db, "Kunde A").await;
    let user = seed_user(db, Some(&account.id), "max@example.org", Role::Member).await;

    let body = json!({"email": " Max@Example.org ", "password": PASSWORD});
    let resp = app.call("POST", "/api/auth/login", None, body).await;
    assert_eq!(resp.status, 200);
    let set_cookie = resp.header("set-cookie").unwrap();
    for attr in ["HttpOnly", "SameSite=Lax", "Secure", "Path=/"] {
        assert!(set_cookie.contains(attr), "{set_cookie}");
    }
    assert_eq!(resp.body["user"]["id"], user.id.as_str());
    assert_eq!(resp.body["user"]["role"], "member");
    assert_eq!(resp.body["account"]["name"], "Kunde A");
    assert_eq!(resp.body["impersonating"], false);
    let cookie = resp.session_cookie().unwrap();

    let session = app.get("/api/auth/session", Some(&cookie)).await;
    assert_eq!(session.status, 200);
    assert_eq!(session.header("cache-control"), Some("no-store"));
    assert_eq!(session.body["user"]["email"], "max@example.org");

    let out = app
        .call("POST", "/api/auth/logout", Some(&cookie), Value::Null)
        .await;
    assert_eq!(out.status, 204);
    assert!(out.header("set-cookie").unwrap().contains("Max-Age=0"));
    let after = app.get("/api/auth/session", Some(&cookie)).await;
    assert_eq!(after.status, 401);
    assert_eq!(after.body["code"], "unauthenticated");
}
dual_test!(login_session_logout_works, login_session_logout);

/// Wrong password and unknown address give the same generic answer; a new login
/// rotates the session.
async fn generic_failure_and_rotation(db: &Db) {
    let app = TestApp::start(db).await;
    let account = seed_account(db, "Kunde").await;
    seed_user(db, Some(&account.id), "a@example.org", Role::Member).await;

    let wrong = json!({"email": "a@example.org", "password": "falsch-falsch"});
    let unknown = json!({"email": "niemand@example.org", "password": "falsch-falsch"});
    let r1 = app.call("POST", "/api/auth/login", None, wrong).await;
    let r2 = app.call("POST", "/api/auth/login", None, unknown).await;
    assert_eq!((r1.status, r2.status), (401, 401));
    assert_eq!(r1.body, r2.body);
    assert_eq!(r1.body["error"], "Anmeldung fehlgeschlagen");

    let first = app.login("a@example.org", PASSWORD).await;
    let body = json!({"email": "a@example.org", "password": PASSWORD});
    let second = app
        .call("POST", "/api/auth/login", Some(&first), body)
        .await;
    let second = second.session_cookie().unwrap();
    assert_ne!(first, second);
    assert_eq!(app.get("/api/auth/session", Some(&first)).await.status, 401);
    assert_eq!(
        app.get("/api/auth/session", Some(&second)).await.status,
        200
    );
}
dual_test!(
    generic_failure_and_rotation_works,
    generic_failure_and_rotation
);

/// A legacy bcrypt hash is accepted once and replaced by argon2id.
async fn bcrypt_upgrade(db: &Db) {
    let app = TestApp::start(db).await;
    let account = seed_account(db, "Kunde").await;
    let legacy = PasswordHash::new(bcrypt::hash(PASSWORD, 4).unwrap());
    seed_user_with_hash(
        db,
        Some(&account.id),
        "alt@example.org",
        Role::Member,
        legacy,
    )
    .await;

    app.login("alt@example.org", PASSWORD).await;
    let upgraded = stored_hash(db, "alt@example.org").await;
    assert!(upgraded.starts_with("$argon2id$"), "{upgraded}");
    app.login("alt@example.org", PASSWORD).await;
}
dual_test!(bcrypt_is_upgraded_to_argon2id, bcrypt_upgrade);

/// A password change needs the current password and ends the other sessions only.
async fn password_change(db: &Db) {
    let app = TestApp::start(db).await;
    let account = seed_account(db, "Kunde").await;
    seed_user(db, Some(&account.id), "p@example.org", Role::Member).await;
    let laptop = app.login("p@example.org", PASSWORD).await;
    let phone = app.login("p@example.org", PASSWORD).await;

    let wrong = json!({"currentPassword": "nicht-richtig", "newPassword": "ganz-neues-passwort"});
    let resp = app.call("PUT", "/api/profile", Some(&laptop), wrong).await;
    assert_eq!(resp.status, 400);
    assert_eq!(
        resp.body["fields"]["currentPassword"],
        "Aktuelles Passwort ist falsch"
    );

    let short = json!({"currentPassword": PASSWORD, "newPassword": "kurz"});
    let resp = app.call("PUT", "/api/profile", Some(&laptop), short).await;
    assert_eq!(resp.status, 400);
    assert_eq!(resp.body["code"], "validation");

    let ok = json!({"name": "  Neuer Name ", "currentPassword": PASSWORD, "newPassword": "ganz-neues-passwort"});
    let resp = app.call("PUT", "/api/profile", Some(&laptop), ok).await;
    assert_eq!(resp.status, 200, "{:?}", resp.body);
    assert_eq!(resp.body["name"], "Neuer Name");
    assert_eq!(app.get("/api/profile", Some(&laptop)).await.status, 200);
    assert_eq!(app.get("/api/profile", Some(&phone)).await.status, 401);
    app.login("p@example.org", "ganz-neues-passwort").await;
}
dual_test!(password_change_ends_other_sessions, password_change);

/// Mutating requests without the CSRF header or from a foreign origin are refused.
#[tokio::test]
async fn csrf_guard() {
    let test_db = vermieterme_db::testing::TestDb::sqlite().await;
    let app = TestApp::start(test_db.db()).await;
    let body = json!({"email": "x@example.org", "password": "irgendwas-langes"});
    let missing = app
        .send(
            "POST",
            "/api/auth/login",
            None,
            Some(body.clone()),
            &[("x-csrf-guard", "")],
        )
        .await;
    assert_eq!(missing.status, 403);
    assert_eq!(
        missing.body,
        json!({"error": "Anfrage abgelehnt", "code": "csrf"})
    );
    let foreign = app
        .send(
            "POST",
            "/api/auth/login",
            None,
            Some(body),
            &[("origin", "https://evil.example")],
        )
        .await;
    assert_eq!(foreign.status, 403);
    assert_eq!(app.get("/api/health", None).await.status, 200);
}

/// After five failures for one address the login answers 429 with `Retry-After`, even
/// with the right password.
#[tokio::test]
async fn login_rate_limit() {
    let test_db = vermieterme_db::testing::TestDb::sqlite().await;
    let db = test_db.db();
    let app = TestApp::start(db).await;
    let account = seed_account(db, "Kunde").await;
    seed_user(db, Some(&account.id), "r@example.org", Role::Member).await;
    let wrong = json!({"email": "r@example.org", "password": "falsch-falsch"});
    for _ in 0..5 {
        let resp = app
            .call("POST", "/api/auth/login", None, wrong.clone())
            .await;
        assert_eq!(resp.status, 401);
    }
    let right = json!({"email": "r@example.org", "password": PASSWORD});
    let resp = app.call("POST", "/api/auth/login", None, right).await;
    assert_eq!(resp.status, 429);
    assert_eq!(resp.body["code"], "rate_limited");
    assert!(resp.header("retry-after").is_some());
}

/// Health and security headers.
#[tokio::test]
async fn health_and_headers() {
    let test_db = vermieterme_db::testing::TestDb::sqlite().await;
    let app = TestApp::start(test_db.db()).await;
    let resp = app.get("/api/health", None).await;
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body["status"], "ok");
    assert_eq!(resp.body["version"], env!("CARGO_PKG_VERSION"));
    let csp = resp.header("content-security-policy").unwrap();
    assert!(csp.contains("frame-ancestors 'none'") && csp.contains("script-src 'self'"));
    assert_eq!(resp.header("x-content-type-options"), Some("nosniff"));
}
