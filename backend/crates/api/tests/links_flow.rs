//! Invitations, reset links and the admin bootstrap over the real router.

// Test bodies and helpers are plain async fns run by `dual_test!`, so clippy's in-test
// allowance for `unwrap` does not cover them (same convention as the db crate's tests).
#![allow(clippy::unwrap_used)]

mod support;

use serde_json::{Value, json};
use support::{PASSWORD, TestApp, config, seed_account, seed_user};
use vermieterme_api::auth::bootstrap::{self, Outcome};
use vermieterme_db::repo::user::Role;
use vermieterme_db::{Db, dual_test};

/// Token from a link like `/accept-invite#token=…`.
fn token_of(url: &Value) -> String {
    let url = url.as_str().unwrap();
    assert!(!url.contains('?'), "token must not be in the query: {url}");
    url.split_once("#token=").unwrap().1.to_owned()
}

/// A member invites a colleague; the link sets name and password once and signs in.
async fn invitation(db: &Db) {
    let app = TestApp::start(db).await;
    let account = seed_account(db, "Kunde").await;
    seed_user(db, Some(&account.id), "chef@example.org", Role::Member).await;
    let cookie = app.login("chef@example.org", PASSWORD).await;

    let body = json!({"name": "Erika", "email": " Erika@Example.org "});
    let resp = app
        .call(
            "POST",
            "/api/users/invitations",
            Some(&cookie),
            body.clone(),
        )
        .await;
    assert_eq!(resp.status, 201, "{:?}", resp.body);
    assert_eq!(resp.body["user"]["email"], "erika@example.org");
    assert!(resp.body["expiresAt"].as_str().unwrap().ends_with('Z'));
    let token = token_of(&resp.body["inviteUrl"]);

    let again = app
        .call("POST", "/api/users/invitations", Some(&cookie), body)
        .await;
    assert_eq!(again.status, 409);
    assert_eq!(again.body["error"], "E-Mail existiert bereits");

    let short = json!({"token": token, "name": "Erika M.", "password": "kurz"});
    assert_eq!(
        app.call("POST", "/api/auth/accept-invite", None, short)
            .await
            .status,
        400
    );
    let accept = json!({"token": token, "name": " Erika M. ", "password": "erikas-passwort"});
    let resp = app
        .call("POST", "/api/auth/accept-invite", None, accept.clone())
        .await;
    assert_eq!(resp.status, 200, "{:?}", resp.body);
    assert_eq!(resp.body["user"]["name"], "Erika M.");
    assert!(resp.session_cookie().is_some());
    let reused = app
        .call("POST", "/api/auth/accept-invite", None, accept)
        .await;
    assert_eq!(reused.status, 401);
    app.login("erika@example.org", "erikas-passwort").await;

    let users = app.get("/api/users", Some(&cookie)).await;
    assert_eq!(users.body.as_array().unwrap().len(), 2);
}
dual_test!(invitation_works_once, invitation);

/// A reset link sets a new password and ends all sessions of the user.
async fn reset(db: &Db) {
    let app = TestApp::start(db).await;
    let account = seed_account(db, "Kunde").await;
    seed_user(db, Some(&account.id), "a@example.org", Role::Member).await;
    let target = seed_user(db, Some(&account.id), "b@example.org", Role::Member).await;
    let cookie_a = app.login("a@example.org", PASSWORD).await;
    let cookie_b = app.login("b@example.org", PASSWORD).await;

    let path = format!("/api/users/{}/reset-link", target.id);
    let resp = app.call("POST", &path, Some(&cookie_a), Value::Null).await;
    assert_eq!(resp.status, 201);
    let token = token_of(&resp.body["resetUrl"]);

    let body = json!({"token": token, "password": "neues-passwort-b"});
    let resp = app
        .call("POST", "/api/auth/reset-password", None, body.clone())
        .await;
    assert_eq!(resp.status, 204);
    assert_eq!(
        app.get("/api/auth/session", Some(&cookie_b)).await.status,
        401
    );
    assert_eq!(
        app.call("POST", "/api/auth/reset-password", None, body)
            .await
            .status,
        401
    );
    app.login("b@example.org", "neues-passwort-b").await;
}
dual_test!(reset_link_ends_sessions, reset);

/// The bootstrap creates the first admin once; after a password change the environment
/// password no longer signs in (#69).
async fn admin_bootstrap(db: &Db) {
    let env = [
        ("ADMIN_EMAIL", "Admin@Example.org"),
        ("ADMIN_PASSWORD", "start-passwort-1"),
    ];
    let cfg = config(&env);
    assert_eq!(bootstrap::run(db, &cfg).await.unwrap(), Outcome::Created);
    assert_eq!(
        bootstrap::run(db, &cfg).await.unwrap(),
        Outcome::AdminExists
    );

    let app = TestApp::start_with(db, cfg).await;
    let cookie = app.login("admin@example.org", "start-passwort-1").await;
    let session = app.get("/api/auth/session", Some(&cookie)).await;
    assert_eq!(session.body["user"]["role"], "admin");
    assert_eq!(session.body["account"], Value::Null);

    let change = json!({"currentPassword": "start-passwort-1", "newPassword": "eigenes-passwort"});
    assert_eq!(
        app.call("PUT", "/api/profile", Some(&cookie), change)
            .await
            .status,
        200
    );
    let old = json!({"email": "admin@example.org", "password": "start-passwort-1"});
    assert_eq!(
        app.call("POST", "/api/auth/login", None, old).await.status,
        401
    );
    let stale = json!({"currentPassword": "start-passwort-1", "newPassword": "noch-ein-passwort"});
    assert_eq!(
        app.call("PUT", "/api/profile", Some(&cookie), stale)
            .await
            .status,
        400
    );
}
dual_test!(admin_bootstrap_runs_once, admin_bootstrap);

/// Without configuration the bootstrap does nothing; a too short password aborts it.
#[tokio::test]
async fn bootstrap_rules() {
    let test_db = vermieterme_db::testing::TestDb::sqlite().await;
    let db = test_db.db();
    assert_eq!(
        bootstrap::run(db, &config(&[])).await.unwrap(),
        Outcome::NotConfigured
    );
    let short = config(&[("ADMIN_EMAIL", "a@example.org"), ("ADMIN_PASSWORD", "kurz")]);
    assert!(bootstrap::run(db, &short).await.is_err());
}
