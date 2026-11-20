//! Administration, account switch with audit, permissions and account isolation over
//! the real router.

// Test bodies and helpers are plain async fns run by `dual_test!`, so clippy's in-test
// allowance for `unwrap` does not cover them (same convention as the db crate's tests).
#![allow(clippy::unwrap_used)]

mod support;

use serde_json::{Value, json};
use support::{PASSWORD, TestApp, audit_count, seed_account, seed_user};
use vermieterme_db::repo::user::Role;
use vermieterme_db::{Db, dual_test};

/// Members get 403 on every admin endpoint.
async fn member_is_no_admin(db: &Db) {
    let app = TestApp::start(db).await;
    let account = seed_account(db, "Kunde").await;
    seed_user(db, Some(&account.id), "m@example.org", Role::Member).await;
    let cookie = app.login("m@example.org", PASSWORD).await;
    let paths = [
        ("GET", "/api/admin/accounts"),
        ("POST", "/api/admin/accounts"),
        ("POST", "/api/admin/switch"),
        ("DELETE", "/api/admin/switch"),
        ("GET", "/api/admin/audit-log"),
    ];
    for (method, path) in paths {
        let body = (method == "POST").then(|| json!({"name": "x", "accountId": account.id}));
        let resp = app.send(method, path, Some(&cookie), body, &[]).await;
        assert_eq!(resp.status, 403, "{method} {path}");
        assert_eq!(resp.body["code"], "forbidden");
    }
    assert_eq!(app.get("/api/admin/accounts", None).await.status, 401);
}
dual_test!(members_cannot_use_admin_endpoints, member_is_no_admin);

/// Users of another account behave like missing ones (404).
async fn foreign_users_are_not_found(db: &Db) {
    let app = TestApp::start(db).await;
    let a = seed_account(db, "A").await;
    let b = seed_account(db, "B").await;
    seed_user(db, Some(&a.id), "a@example.org", Role::Member).await;
    let foreign = seed_user(db, Some(&b.id), "b@example.org", Role::Member).await;
    let cookie = app.login("a@example.org", PASSWORD).await;

    let reset = format!("/api/users/{}/reset-link", foreign.id);
    let resp = app.call("POST", &reset, Some(&cookie), Value::Null).await;
    assert_eq!(resp.status, 404);
    assert_eq!(resp.body["code"], "not_found");
    let delete = format!("/api/users/{}", foreign.id);
    assert_eq!(
        app.call("DELETE", &delete, Some(&cookie), Value::Null)
            .await
            .status,
        404
    );
    let users = app.get("/api/users", Some(&cookie)).await;
    assert_eq!(users.body.as_array().unwrap().len(), 1);
    let me = users.body[0]["id"].as_str().unwrap().to_owned();
    let own = format!("/api/users/{me}");
    assert_eq!(
        app.call("DELETE", &own, Some(&cookie), Value::Null)
            .await
            .status,
        400
    );
}
dual_test!(foreign_users_give_404, foreign_users_are_not_found);

/// Admin creates an account, invites a user, switches in and out; switch and changes in
/// the foreign account are audited and the session reports `impersonating`.
async fn admin_switch(db: &Db) {
    let app = TestApp::start(db).await;
    let home = seed_account(db, "Eigener Kunde").await;
    seed_user(db, Some(&home.id), "admin@example.org", Role::Admin).await;
    let cookie = app.login("admin@example.org", PASSWORD).await;

    let created = app
        .call(
            "POST",
            "/api/admin/accounts",
            Some(&cookie),
            json!({"name": " Neu "}),
        )
        .await;
    assert_eq!(created.status, 201);
    assert_eq!(created.body["name"], "Neu");
    let id = created.body["id"].as_str().unwrap().to_owned();
    let invite = json!({"name": "Kundin", "email": "kundin@example.org"});
    let path = format!("/api/admin/accounts/{id}/users");
    assert_eq!(
        app.call("POST", &path, Some(&cookie), invite).await.status,
        201
    );
    assert_eq!(
        app.get(&path, Some(&cookie))
            .await
            .body
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let missing = app
        .call(
            "POST",
            "/api/admin/switch",
            Some(&cookie),
            json!({"accountId": "fehlt"}),
        )
        .await;
    assert_eq!(missing.status, 404);

    let resp = app
        .call(
            "POST",
            "/api/admin/switch",
            Some(&cookie),
            json!({"accountId": id}),
        )
        .await;
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body["impersonating"], true);
    assert_eq!(resp.body["account"]["id"], id.as_str());
    assert_eq!(audit_count(db, "account.switch").await, 1);

    let invite = json!({"name": "Zweite", "email": "zweite@example.org"});
    assert_eq!(
        app.call("POST", "/api/users/invitations", Some(&cookie), invite)
            .await
            .status,
        201
    );
    assert_eq!(audit_count(db, "user.invite").await, 2);
    assert_eq!(
        app.get("/api/users", Some(&cookie))
            .await
            .body
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let back = app
        .send("DELETE", "/api/admin/switch", Some(&cookie), None, &[])
        .await;
    assert_eq!(back.body["impersonating"], false);
    assert_eq!(back.body["account"]["id"], home.id.as_str());
    assert_eq!(audit_count(db, "account.switch_back").await, 1);

    let log = app
        .get(
            &format!("/api/admin/audit-log?accountId={id}"),
            Some(&cookie),
        )
        .await;
    let items = log.body["items"].as_array().unwrap();
    assert_eq!(items.len(), 5);
    assert_eq!(items[0]["action"], "account.switch_back");
    assert_eq!(items[0]["userEmail"], "admin@example.org");
    assert_eq!(log.body["nextCursor"], serde_json::Value::Null);
    let bad = app
        .get("/api/admin/audit-log?cursor=%21%21", Some(&cookie))
        .await;
    assert_eq!(bad.status, 400);
}
dual_test!(admin_switch_is_audited, admin_switch);

/// Locking an account ends its members' sessions; admins cannot change themselves.
async fn lock_account(db: &Db) {
    let app = TestApp::start(db).await;
    let account = seed_account(db, "Kunde").await;
    let admin = seed_user(db, None, "admin@example.org", Role::Admin).await;
    let member = seed_user(db, Some(&account.id), "m@example.org", Role::Member).await;
    let admin_cookie = app.login("admin@example.org", PASSWORD).await;
    let member_cookie = app.login("m@example.org", PASSWORD).await;

    let path = format!("/api/admin/accounts/{}", account.id);
    let resp = app
        .call("PATCH", &path, Some(&admin_cookie), json!({"locked": true}))
        .await;
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body["locked"], true);
    assert_eq!(
        app.get("/api/auth/session", Some(&member_cookie))
            .await
            .status,
        401
    );
    let login = json!({"email": "m@example.org", "password": PASSWORD});
    assert_eq!(
        app.call("POST", "/api/auth/login", None, login)
            .await
            .status,
        401
    );
    assert_eq!(audit_count(db, "account.lock").await, 1);

    let own = format!("/api/admin/users/{}", admin.id);
    let resp = app
        .call("PATCH", &own, Some(&admin_cookie), json!({"locked": true}))
        .await;
    assert_eq!(resp.status, 400);
    let other = format!("/api/admin/users/{}", member.id);
    let resp = app
        .call(
            "PATCH",
            &other,
            Some(&admin_cookie),
            json!({"role": "chef"}),
        )
        .await;
    assert_eq!(resp.status, 400);
    let missing = app
        .call(
            "PATCH",
            "/api/admin/accounts/fehlt",
            Some(&admin_cookie),
            json!({}),
        )
        .await;
    assert_eq!(missing.status, 404);
    let session = app.get("/api/auth/session", Some(&admin_cookie)).await;
    assert_eq!(session.body["account"], serde_json::Value::Null);
}
dual_test!(account_lock_ends_member_sessions, lock_account);
