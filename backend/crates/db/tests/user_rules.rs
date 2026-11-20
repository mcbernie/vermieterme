//! Rules of user changes: a member always belongs to an account (repository and schema,
//! `app_user_member_account_chk`); the last active admin stays; role changes and locks
//! end the user's sessions.

// Test bodies are plain async fns run by `dual_test!`, so clippy's in-test allowance
// does not reach them.
#![allow(clippy::unwrap_used)]
// Tests build scopes directly (no session layer here); see vermieterme_db::scope.
#![allow(clippy::disallowed_methods)]

#[path = "support/fixtures.rs"]
mod fixtures;

use fixtures::{HOUR, NOW, setup};
use sqlx::AnyConnection;
use vermieterme_db::repo::session;
use vermieterme_db::repo::user::{self, MemberPatch, NewUser, Role, UserPatch};
use vermieterme_db::{AdminScope, Db, DbError, SecretToken, dual_test};

/// A new user with `role` and no password.
fn new_user(email: &str, role: Role) -> NewUser {
    NewUser {
        email: email.into(),
        name: "N".into(),
        role,
        password_hash: None,
    }
}

/// Members without account are refused on create, on demotion and in raw SQL.
async fn member_needs_account(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let admin = AdminScope::system();

    let err = user::create_for_admin(
        &mut c,
        &admin,
        None,
        &new_user("m@example.org", Role::Member),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, DbError::Constraint), "{err:?}");

    let root = user::create_for_admin(
        &mut c,
        &admin,
        None,
        &new_user("a@example.org", Role::Admin),
    )
    .await
    .unwrap();
    // A second active admin, so the last-admin guard does not answer first.
    setup(&mut c, "b@example.org", Role::Admin).await;
    let demote = UserPatch {
        role: Some(Role::Member),
        ..UserPatch::default()
    };
    let err = user::update_for_admin(&mut c, &admin, &root.id, &demote)
        .await
        .unwrap_err();
    assert!(matches!(err, DbError::Constraint), "{err:?}");
    let unchanged = user::get_for_admin(&mut c, &admin, &root.id).await.unwrap();
    assert_eq!(unchanged.role, Role::Admin);

    let err: DbError = sqlx::query(
        "INSERT INTO app_user (id, account_id, email, name, role, locked, created_at, \
         updated_at) VALUES ('u-raw', NULL, 'raw@example.org', 'N', 'member', 0, 1, 1)",
    )
    .execute(&mut *c)
    .await
    .unwrap_err()
    .into();
    assert!(matches!(err, DbError::Constraint), "Schema: {err:?}");
}
dual_test!(user_member_account, member_needs_account);

/// Patch changing only the role.
fn role(role: Role) -> UserPatch {
    UserPatch {
        role: Some(role),
        ..UserPatch::default()
    }
}

/// Patch changing only the lock state.
fn lock(locked: bool) -> UserPatch {
    UserPatch {
        locked: Some(locked),
        ..UserPatch::default()
    }
}

/// The last active admin can be neither demoted nor locked; a locked admin does not count.
async fn last_admin_stays(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let admin = AdminScope::system();
    let (_, a) = setup(&mut c, "a@example.org", Role::Admin).await;
    for patch in [role(Role::Member), lock(true)] {
        let err = user::update_for_admin(&mut c, &admin, &a, &patch)
            .await
            .unwrap_err();
        assert!(matches!(err, DbError::Conflict), "{err:?}");
    }
    let (_, b) = setup(&mut c, "b@example.org", Role::Admin).await;
    user::update_for_admin(&mut c, &admin, &b, &lock(true))
        .await
        .unwrap();
    let err = user::update_for_admin(&mut c, &admin, &a, &role(Role::Member))
        .await
        .unwrap_err();
    assert!(
        matches!(err, DbError::Conflict),
        "gesperrter Admin zählt nicht"
    );
    let unchanged = user::get_for_admin(&mut c, &admin, &a).await.unwrap();
    assert_eq!((unchanged.role, unchanged.locked), (Role::Admin, false));
    // Renaming the last admin while keeping role and lock state is fine.
    let rename = UserPatch {
        name: Some("Chef".into()),
        role: Some(Role::Admin),
        locked: Some(false),
    };
    user::update_for_admin(&mut c, &admin, &a, &rename)
        .await
        .unwrap();

    user::update_for_admin(&mut c, &admin, &b, &lock(false))
        .await
        .unwrap();
    let demoted = user::update_for_admin(&mut c, &admin, &a, &role(Role::Member))
        .await
        .unwrap();
    assert_eq!(demoted.role, Role::Member);
}
dual_test!(user_last_admin, last_admin_stays);

/// Two connections demote the last two active admins at the same time: one admin stays.
async fn last_admin_concurrent(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let admin = AdminScope::system();
    let (_, a) = setup(&mut c, "a@example.org", Role::Admin).await;
    let (_, b) = setup(&mut c, "b@example.org", Role::Admin).await;
    let mut ca = db.pool().acquire().await.unwrap();
    let mut cb = db.pool().acquire().await.unwrap();
    let patch = role(Role::Member);
    let (ra, rb) = tokio::join!(
        user::update_for_admin(&mut ca, &admin, &a, &patch),
        user::update_for_admin(&mut cb, &admin, &b, &patch),
    );
    // The loser gets Conflict (or a lock error); never do both succeed.
    assert!(!(ra.is_ok() && rb.is_ok()), "{ra:?} / {rb:?}");
    assert!(ra.is_ok() || rb.is_ok(), "{ra:?} / {rb:?}");
    let active: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM app_user WHERE role = 'admin' AND locked = 0")
            .fetch_one(&mut *c)
            .await
            .unwrap();
    assert_eq!(active, 1, "genau ein aktiver Admin bleibt");
}
dual_test!(user_last_admin_race, last_admin_concurrent);

/// Opens a session for `user_id`; returns its token.
async fn signed_in(c: &mut AnyConnection, user_id: &str) -> SecretToken {
    let token = SecretToken::generate();
    session::create(c, user_id, &token, NOW, NOW + HOUR)
        .await
        .unwrap();
    token
}

/// Whether the session of `token` is still valid.
async fn valid(c: &mut AnyConnection, token: &SecretToken) -> bool {
    session::find_valid(c, token, NOW, 0)
        .await
        .unwrap()
        .is_some()
}

/// Role changes and locks end all sessions; other changes keep them.
async fn changes_end_sessions(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let admin = AdminScope::system();
    let (scope, a) = setup(&mut c, "a@example.org", Role::Admin).await;
    setup(&mut c, "b@example.org", Role::Admin).await;

    // A switched admin who is demoted loses the session (and with it the switch).
    let token = signed_in(&mut c, &a).await;
    let s = session::find_valid(&mut c, &token, NOW, 0)
        .await
        .unwrap()
        .unwrap();
    session::set_acting_account(&mut c, &admin, &s.session.id, Some(scope.account_id()))
        .await
        .unwrap();
    let rename = UserPatch {
        name: Some("Neu".into()),
        ..UserPatch::default()
    };
    user::update_for_admin(&mut c, &admin, &a, &rename)
        .await
        .unwrap();
    assert!(
        valid(&mut c, &token).await,
        "Name ändern behält die Sitzung"
    );
    user::update_for_admin(&mut c, &admin, &a, &role(Role::Member))
        .await
        .unwrap();
    assert!(
        !valid(&mut c, &token).await,
        "Rollenwechsel beendet die Sitzung"
    );

    // Lock through the administration and through the account scope.
    let token = signed_in(&mut c, &a).await;
    user::update_for_admin(&mut c, &admin, &a, &lock(true))
        .await
        .unwrap();
    user::update_for_admin(&mut c, &admin, &a, &lock(false))
        .await
        .unwrap();
    assert!(!valid(&mut c, &token).await, "Sperre beendet die Sitzung");
    let token = signed_in(&mut c, &a).await;
    let locked = MemberPatch {
        locked: Some(true),
        ..MemberPatch::default()
    };
    user::update(&mut c, &scope, &a, &locked).await.unwrap();
    let unlocked = MemberPatch {
        locked: Some(false),
        ..MemberPatch::default()
    };
    user::update(&mut c, &scope, &a, &unlocked).await.unwrap();
    assert!(
        !valid(&mut c, &token).await,
        "Sperre im Kunden beendet die Sitzung"
    );
}
dual_test!(user_changes_end_sessions, changes_end_sessions);
