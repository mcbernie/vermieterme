//! Membership rule of users: a member always belongs to an account; only an admin may
//! have none. Enforced by the repository and by the schema (`app_user_member_account_chk`).

// Test bodies are plain async fns run by `dual_test!`, so clippy's in-test allowance
// does not reach them.
#![allow(clippy::unwrap_used)]
// Tests build scopes directly (no session layer here); see vermieterme_db::scope.
#![allow(clippy::disallowed_methods)]

use vermieterme_db::repo::user::{self, NewUser, Role, UserPatch};
use vermieterme_db::{AdminScope, Db, DbError, dual_test};

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
