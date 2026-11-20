//! Invitations, reset links and the admin bootstrap over the real router.

// Test bodies and helpers are plain async fns run by `dual_test!`, so clippy's in-test
// allowance for `unwrap` does not cover them (same convention as the db crate's tests).
#![allow(clippy::unwrap_used)]

mod support;

use std::sync::Arc;

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

/// A made-up token is refused (401) without any argon2 work, for both kinds of link.
#[tokio::test]
async fn made_up_token_costs_no_hash() {
    let test_db = vermieterme_db::testing::TestDb::sqlite().await;
    let app = TestApp::start(test_db.db()).await;
    let reset = json!({"token": "erfunden", "password": "neues-passwort-x"});
    let invite = json!({"token": "erfunden", "name": "X", "password": "neues-passwort-x"});
    for (path, body) in [
        ("/api/auth/reset-password", reset),
        ("/api/auth/accept-invite", invite),
    ] {
        let resp = app.call("POST", path, None, body).await;
        assert_eq!(resp.status, 401, "{path}");
        assert_eq!(resp.body["code"], "unauthenticated");
    }
    assert_eq!(app.hasher.computed(), 0);
}

/// With every hashing permit taken the redemption answers 429 after the wait limit, and
/// the link stays usable.
#[tokio::test]
async fn overload_does_not_burn_the_link() {
    let test_db = vermieterme_db::testing::TestDb::sqlite().await;
    let db = test_db.db();
    let app = TestApp::start_with(db, config(&[("PASSWORD_HASH_CONCURRENCY", "1")])).await;
    let account = seed_account(db, "Kunde").await;
    seed_user(db, Some(&account.id), "a@example.org", Role::Member).await;
    let target = seed_user(db, Some(&account.id), "b@example.org", Role::Member).await;
    let cookie = app.login("a@example.org", PASSWORD).await;
    let path = format!("/api/users/{}/reset-link", target.id);
    let resp = app.call("POST", &path, Some(&cookie), Value::Null).await;
    let body = json!({"token": token_of(&resp.body["resetUrl"]), "password": "neues-passwort-b"});

    let held = app.hasher.permit().await.unwrap();
    let busy = app
        .call("POST", "/api/auth/reset-password", None, body.clone())
        .await;
    assert_eq!(busy.status, 429);
    assert!(busy.header("retry-after").is_some());
    drop(held);
    let resp = app
        .call("POST", "/api/auth/reset-password", None, body)
        .await;
    assert_eq!(resp.status, 204);
}

/// Two redemptions of one link pass the check and hash one after the other (outside
/// any transaction); only the first consumes the token, the second gets the generic 401.
async fn parallel_redemption(db: &Db) {
    let cfg = config(&[("PASSWORD_HASH_CONCURRENCY", "1")]);
    let app = Arc::new(TestApp::start_with(db, cfg).await);
    let account = seed_account(db, "Kunde").await;
    seed_user(db, Some(&account.id), "a@example.org", Role::Member).await;
    let target = seed_user(db, Some(&account.id), "b@example.org", Role::Member).await;
    let cookie = app.login("a@example.org", PASSWORD).await;
    let path = format!("/api/users/{}/reset-link", target.id);
    let resp = app.call("POST", &path, Some(&cookie), Value::Null).await;
    let token = token_of(&resp.body["resetUrl"]);

    let held = app.hasher.permit().await.unwrap();
    let tasks: Vec<_> = ["neues-passwort-1", "neues-passwort-2"]
        .into_iter()
        .map(|password| {
            let app = app.clone();
            let body = json!({"token": token, "password": password});
            tokio::spawn(async move {
                app.call("POST", "/api/auth/reset-password", None, body)
                    .await
            })
        })
        .collect();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    drop(held);
    let mut statuses = Vec::new();
    for task in tasks {
        statuses.push(task.await.unwrap().status);
    }
    statuses.sort_unstable();
    assert_eq!(statuses, [204, 401]);
    assert_eq!(app.hasher.computed(), 3, "login plus both redemptions");
}
dual_test!(parallel_redemption_succeeds_once, parallel_redemption);

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
