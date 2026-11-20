//! User administration over the real router: role change and lock end sessions and links,
//! deleting members, reset links, invitations of locked users and accounts, the bootstrap
//! with an invited user.

// Test bodies and helpers are plain async fns run by `dual_test!`, so clippy's in-test
// allowance for `unwrap` does not cover them (same convention as the db crate's tests).
#![allow(clippy::unwrap_used)]

mod support;

use serde_json::{Value, json};
use support::{PASSWORD, TestApp, audit_count, config, seed_account, seed_user};
use vermieterme_api::auth::bootstrap::{self, BootstrapError};
use vermieterme_db::repo::user::{self, Role, UserPatch};
use vermieterme_db::{AdminScope, Db, dual_test};

/// Token from a link like `/reset-password#token=…`.
fn token_of(url: &Value) -> String {
    url.as_str()
        .unwrap()
        .split_once("#token=")
        .unwrap()
        .1
        .to_owned()
}

/// Locks or unlocks a user directly in the database (no link revocation).
#[allow(clippy::disallowed_methods)] // Test set-up, no request involved.
async fn set_locked(db: &Db, user_id: &str, locked: bool) {
    let mut conn = db.pool().acquire().await.unwrap();
    let patch = UserPatch {
        locked: Some(locked),
        ..UserPatch::default()
    };
    user::update_for_admin(&mut conn, &AdminScope::system(), user_id, &patch)
        .await
        .unwrap();
}

/// Redeems a reset link and returns the status.
async fn redeem_reset(app: &TestApp, url: &Value, password: &str) -> u16 {
    let body = json!({"token": token_of(url), "password": password});
    app.call("POST", "/api/auth/reset-password", None, body)
        .await
        .status
}

/// Admin PATCH: a role change and a lock end the sessions of the user; role and lock in
/// one request leave two audit entries; a lock makes open reset links unusable and a
/// locked user cannot sign in; a fresh reset link works after unlocking.
async fn admin_user_changes(db: &Db) {
    let app = TestApp::start(db).await;
    let account = seed_account(db, "Kunde").await;
    seed_user(db, None, "admin@example.org", Role::Admin).await;
    let member = seed_user(db, Some(&account.id), "m@example.org", Role::Member).await;
    let admin = app.login("admin@example.org", PASSWORD).await;
    let path = format!("/api/admin/users/{}", member.id);

    let cookie = app.login("m@example.org", PASSWORD).await;
    let resp = app
        .call("PATCH", &path, Some(&admin), json!({"role": "admin"}))
        .await;
    assert_eq!(resp.status, 200, "{:?}", resp.body);
    assert_eq!(
        app.get("/api/auth/session", Some(&cookie)).await.status,
        401
    );

    let reset = format!("/api/admin/users/{}/reset-link", member.id);
    let link = app.call("POST", &reset, Some(&admin), Value::Null).await;
    assert_eq!(link.status, 201);
    assert_eq!(audit_count(db, "user.reset_link").await, 1);
    let cookie = app.login("m@example.org", PASSWORD).await;
    let both = json!({"role": "member", "locked": true});
    let resp = app.call("PATCH", &path, Some(&admin), both).await;
    assert_eq!(resp.status, 200, "{:?}", resp.body);
    assert_eq!(
        app.get("/api/auth/session", Some(&cookie)).await.status,
        401
    );
    assert_eq!(audit_count(db, "user.role").await, 2);
    assert_eq!(audit_count(db, "user.lock").await, 1);
    let login = json!({"email": "m@example.org", "password": PASSWORD});
    let resp = app.call("POST", "/api/auth/login", None, login).await;
    assert_eq!(resp.status, 401);

    let unlock = app
        .call("PATCH", &path, Some(&admin), json!({"locked": false}))
        .await;
    assert_eq!(unlock.status, 200);
    assert_eq!(
        redeem_reset(&app, &link.body["resetUrl"], "neues-passwort-m").await,
        401
    );
    let fresh = app.call("POST", &reset, Some(&admin), Value::Null).await;
    assert_eq!(
        redeem_reset(&app, &fresh.body["resetUrl"], "neues-passwort-m").await,
        204
    );
    app.login("m@example.org", "neues-passwort-m").await;
}
dual_test!(
    admin_user_changes_end_sessions_and_links,
    admin_user_changes
);

/// A member deletes a colleague (204, no more sign-in) but can neither delete the admin
/// of the own account nor issue a reset link for them (404, like a foreign user).
async fn member_deletes(db: &Db) {
    let app = TestApp::start(db).await;
    let account = seed_account(db, "Kunde").await;
    seed_user(db, Some(&account.id), "a@example.org", Role::Member).await;
    let other = seed_user(db, Some(&account.id), "b@example.org", Role::Member).await;
    let admin = seed_user(db, Some(&account.id), "admin@example.org", Role::Admin).await;
    let cookie = app.login("a@example.org", PASSWORD).await;
    let other_cookie = app.login("b@example.org", PASSWORD).await;

    let path = format!("/api/users/{}", other.id);
    let resp = app.call("DELETE", &path, Some(&cookie), Value::Null).await;
    assert_eq!(resp.status, 204);
    assert_eq!(
        app.get("/api/auth/session", Some(&other_cookie))
            .await
            .status,
        401
    );
    let login = json!({"email": "b@example.org", "password": PASSWORD});
    assert_eq!(
        app.call("POST", "/api/auth/login", None, login)
            .await
            .status,
        401
    );

    let path = format!("/api/users/{}", admin.id);
    let resp = app.call("DELETE", &path, Some(&cookie), Value::Null).await;
    assert_eq!(resp.status, 404);
    let path = format!("/api/users/{}/reset-link", admin.id);
    let resp = app.call("POST", &path, Some(&cookie), Value::Null).await;
    assert_eq!(resp.status, 404);
    app.login("admin@example.org", PASSWORD).await;
}
dual_test!(member_deletes_colleague_but_not_admin, member_deletes);

/// Accepting the invitation of a locked user fails (401) and leaves the link unused: after
/// unlocking it works.
async fn invite_locked(db: &Db) {
    let app = TestApp::start(db).await;
    let account = seed_account(db, "Kunde").await;
    seed_user(db, Some(&account.id), "chef@example.org", Role::Member).await;
    let cookie = app.login("chef@example.org", PASSWORD).await;
    let body = json!({"name": "Neu", "email": "neu@example.org"});
    let resp = app
        .call("POST", "/api/users/invitations", Some(&cookie), body)
        .await;
    assert_eq!(resp.status, 201);
    let invited = resp.body["user"]["id"].as_str().unwrap().to_owned();
    let token = token_of(&resp.body["inviteUrl"]);
    let accept = json!({"token": token, "name": "Neu", "password": "neues-passwort-1"});

    set_locked(db, &invited, true).await;
    let resp = app
        .call("POST", "/api/auth/accept-invite", None, accept.clone())
        .await;
    assert_eq!(resp.status, 401);
    assert!(resp.session_cookie().is_none());
    set_locked(db, &invited, false).await;
    let resp = app
        .call("POST", "/api/auth/accept-invite", None, accept)
        .await;
    assert_eq!(resp.status, 200, "{:?}", resp.body);
}
dual_test!(invitation_of_locked_user_stays_unused, invite_locked);

/// Links of a member of a locked account are refused (401) and stay unused (#105): after
/// unlocking, the same reset link and invitation work.
async fn account_locked_links(db: &Db) {
    let app = TestApp::start(db).await;
    let account = seed_account(db, "Kunde").await;
    seed_user(db, None, "admin@example.org", Role::Admin).await;
    seed_user(db, Some(&account.id), "c@example.org", Role::Member).await;
    let chef = app.login("c@example.org", PASSWORD).await;
    let member = seed_user(db, Some(&account.id), "m@example.org", Role::Member).await;
    let path = format!("/api/users/{}/reset-link", member.id);
    let reset = app.call("POST", &path, Some(&chef), Value::Null).await;
    let body = json!({"name": "Neu", "email": "neu@example.org"});
    let invite = app
        .call("POST", "/api/users/invitations", Some(&chef), body)
        .await;
    let accept = json!({"token": token_of(&invite.body["inviteUrl"]), "name": "Neu",
        "password": "neues-passwort-1"});
    let admin = app.login("admin@example.org", PASSWORD).await;
    let lock = async |locked: bool| {
        let path = format!("/api/admin/accounts/{}", account.id);
        let resp = app
            .call("PATCH", &path, Some(&admin), json!({"locked": locked}))
            .await;
        assert_eq!(resp.status, 200, "{:?}", resp.body);
    };

    lock(true).await;
    let url = &reset.body["resetUrl"];
    assert_eq!(redeem_reset(&app, url, "neues-passwort-m").await, 401);
    let resp = app
        .call("POST", "/api/auth/accept-invite", None, accept.clone())
        .await;
    assert_eq!(resp.status, 401);
    lock(false).await;
    assert_eq!(redeem_reset(&app, url, "neues-passwort-m").await, 204);
    app.login("m@example.org", "neues-passwort-m").await;
    let resp = app
        .call("POST", "/api/auth/accept-invite", None, accept)
        .await;
    assert_eq!(resp.status, 200, "{:?}", resp.body);
}
dual_test!(links_of_locked_account_stay_unused, account_locked_links);

/// A password change makes an open reset link unusable.
async fn password_change_revokes(db: &Db) {
    let app = TestApp::start(db).await;
    let account = seed_account(db, "Kunde").await;
    seed_user(db, Some(&account.id), "a@example.org", Role::Member).await;
    let target = seed_user(db, Some(&account.id), "b@example.org", Role::Member).await;
    let cookie_a = app.login("a@example.org", PASSWORD).await;
    let path = format!("/api/users/{}/reset-link", target.id);
    let link = app.call("POST", &path, Some(&cookie_a), Value::Null).await;
    assert_eq!(link.status, 201);

    let cookie_b = app.login("b@example.org", PASSWORD).await;
    let change = json!({"currentPassword": PASSWORD, "newPassword": "eigenes-passwort-b"});
    let resp = app
        .call("PUT", "/api/profile", Some(&cookie_b), change)
        .await;
    assert_eq!(resp.status, 200);
    assert_eq!(
        redeem_reset(&app, &link.body["resetUrl"], "fremdes-passwort-b").await,
        401
    );
    app.login("b@example.org", "eigenes-passwort-b").await;
}
dual_test!(password_change_revokes_reset_links, password_change_revokes);

/// `ADMIN_EMAIL` of an invited user without password stops the start instead of
/// promoting them (the holder of the invitation would choose the admin password).
async fn bootstrap_invited(db: &Db) {
    let account = seed_account(db, "Kunde").await;
    let app = TestApp::start(db).await;
    seed_user(db, Some(&account.id), "chef@example.org", Role::Member).await;
    let cookie = app.login("chef@example.org", PASSWORD).await;
    let body = json!({"name": "Neu", "email": "neu@example.org"});
    let resp = app
        .call("POST", "/api/users/invitations", Some(&cookie), body)
        .await;
    assert_eq!(resp.status, 201);

    let env = [
        ("ADMIN_EMAIL", "neu@example.org"),
        ("ADMIN_PASSWORD", "start-passwort-1"),
    ];
    let result = bootstrap::run(db, &config(&env)).await;
    assert!(
        matches!(result, Err(BootstrapError::InvitedUser)),
        "{result:?}"
    );
    let users = app.get("/api/users", Some(&cookie)).await;
    let invited = users
        .body
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == "neu@example.org")
        .unwrap()
        .clone();
    assert_eq!(invited["role"], "member");
}
dual_test!(bootstrap_refuses_invited_user, bootstrap_invited);
