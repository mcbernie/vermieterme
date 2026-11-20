//! Repositories for accounts and users, including scope isolation (foreign id -> NotFound).

// Test bodies are plain async fns run by `dual_test!`, so clippy's in-test allowance
// does not reach them.
#![allow(clippy::unwrap_used)]
// Tests build scopes directly (no session layer here); see vermieterme_db::scope.
#![allow(clippy::disallowed_methods)]

use vermieterme_db::repo::account::{self, AccountPatch};
use vermieterme_db::repo::user::{self, MemberPatch, NewMember, NewUser, Role, UserPatch};
use vermieterme_db::{AdminScope, Db, DbError, PasswordHash, UserScope, dual_test};

/// A new member with the given address and no password (invitation).
fn member(email: &str) -> NewMember {
    NewMember {
        email: email.into(),
        name: "Erika Muster".into(),
        password_hash: None,
    }
}

/// Create, read, list with user count, partial update, unknown id.
async fn account_lifecycle(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let admin = AdminScope::system();
    assert_eq!(account::first(&mut c, &admin).await.unwrap(), None);
    let a = account::create(&mut c, &admin, "Kunde A").await.unwrap();
    let b = account::create(&mut c, &admin, "Kunde B").await.unwrap();
    assert!(!a.locked);
    assert_eq!(account::get(&mut c, &admin, &a.id).await.unwrap(), a);
    assert_eq!(
        account::get_own(&mut c, &admin.act_in(&b.id))
            .await
            .unwrap(),
        b
    );

    user::create(&mut c, &admin.act_in(&a.id), &member("e@example.org"))
        .await
        .unwrap();
    let list = account::list(&mut c, &admin).await.unwrap();
    let counts: Vec<(&str, i64)> = list
        .iter()
        .map(|s| (s.account.name.as_str(), s.user_count))
        .collect();
    assert_eq!(counts, [("Kunde A", 1), ("Kunde B", 0)]);

    let patch = AccountPatch {
        name: None,
        locked: Some(true),
    };
    let locked = account::update(&mut c, &admin, &a.id, &patch)
        .await
        .unwrap();
    assert!(locked.locked);
    assert_eq!(locked.name, "Kunde A", "nicht gesendetes Feld bleibt");
    let patch = AccountPatch {
        name: Some("Kunde A2".into()),
        locked: None,
    };
    let renamed = account::update(&mut c, &admin, &a.id, &patch)
        .await
        .unwrap();
    assert_eq!((renamed.name.as_str(), renamed.locked), ("Kunde A2", true));

    assert!(matches!(
        account::get(&mut c, &admin, "fehlt").await,
        Err(DbError::NotFound)
    ));
    let err = account::update(&mut c, &admin, "fehlt", &AccountPatch::default()).await;
    assert!(matches!(err, Err(DbError::NotFound)));
    assert_eq!(
        account::first(&mut c, &admin).await.unwrap().map(|x| x.id),
        Some(a.id)
    );
}
dual_test!(account_crud, account_lifecycle);

/// Users of account A are invisible and unchangeable through the scope of account B.
async fn user_scope_isolation(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let admin = AdminScope::system();
    let a = admin.act_in(account::create(&mut c, &admin, "A").await.unwrap().id);
    let b = admin.act_in(account::create(&mut c, &admin, "B").await.unwrap().id);
    let u = user::create(&mut c, &a, &member("  Erika@Example.ORG "))
        .await
        .unwrap();
    assert_eq!(u.email, "erika@example.org");
    assert_eq!(u.account_id.as_deref(), Some(a.account_id()));
    assert!(!u.has_password);

    assert_eq!(user::get(&mut c, &a, &u.id).await.unwrap(), u);
    assert!(matches!(
        user::get(&mut c, &b, &u.id).await,
        Err(DbError::NotFound)
    ));
    assert!(user::list(&mut c, &b).await.unwrap().is_empty());
    let patch = MemberPatch {
        locked: Some(true),
        ..MemberPatch::default()
    };
    assert!(matches!(
        user::update(&mut c, &b, &u.id, &patch).await,
        Err(DbError::NotFound)
    ));
    assert!(matches!(
        user::delete(&mut c, &b, &u.id).await,
        Err(DbError::NotFound)
    ));
    assert!(
        !user::get(&mut c, &a, &u.id).await.unwrap().locked,
        "unverändert"
    );

    let changed = user::update(&mut c, &a, &u.id, &patch).await.unwrap();
    assert!(changed.locked);
    assert_eq!(changed.role, Role::Member);
    user::delete(&mut c, &a, &u.id).await.unwrap();
    assert!(matches!(
        user::get(&mut c, &a, &u.id).await,
        Err(DbError::NotFound)
    ));
}
dual_test!(user_isolation, user_scope_isolation);

/// E-mail addresses are unique regardless of case; violations become `Conflict`.
async fn duplicate_email_conflicts(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let admin = AdminScope::system();
    let a = admin.act_in(account::create(&mut c, &admin, "A").await.unwrap().id);
    let b = admin.act_in(account::create(&mut c, &admin, "B").await.unwrap().id);
    user::create(&mut c, &a, &member("max@example.org"))
        .await
        .unwrap();
    let err = user::create(&mut c, &b, &member("MAX@example.org")).await;
    assert!(matches!(err, Err(DbError::Conflict)));
}
dual_test!(user_duplicate_email, duplicate_email_conflicts);

/// Login lookup, password change of the own user, admin bootstrap and admin updates.
async fn login_and_admin(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let system = AdminScope::system();
    assert!(!user::admin_exists(&mut c).await.unwrap());
    let new = NewUser {
        email: "Admin@Example.org".into(),
        name: "Admin".into(),
        role: Role::Admin,
        password_hash: Some(PasswordHash::new("$2b$10$legacy")),
    };
    let admin = user::create_for_admin(&mut c, &system, None, &new)
        .await
        .unwrap();
    assert_eq!(
        (admin.role, admin.account_id.as_deref()),
        (Role::Admin, None)
    );
    assert!(user::admin_exists(&mut c).await.unwrap());

    let login = user::find_login(&mut c, " admin@EXAMPLE.org")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(login.user.id, admin.id);
    assert_eq!(
        login.password_hash.as_ref().map(PasswordHash::as_str),
        Some("$2b$10$legacy")
    );
    assert!(
        !format!("{login:?}").contains("legacy"),
        "Hash nie im Debug-Text"
    );
    assert!(
        user::find_login(&mut c, "niemand@example.org")
            .await
            .unwrap()
            .is_none()
    );

    let me = UserScope::from_session(&admin.id);
    user::set_password_hash(&mut c, &me, &PasswordHash::new("$argon2id$neu"))
        .await
        .unwrap();
    let login = user::find_login(&mut c, "admin@example.org")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(login.password_hash.unwrap().as_str(), "$argon2id$neu");
    assert_eq!(
        user::set_own_name(&mut c, &me, "Chef").await.unwrap().name,
        "Chef"
    );
    assert_eq!(user::get_self(&mut c, &me).await.unwrap().name, "Chef");

    let verified = AdminScope::verified_admin(&admin.id);
    let acc = verified.act_in(account::create(&mut c, &verified, "A").await.unwrap().id);
    let m = user::create(&mut c, &acc, &member("m@example.org"))
        .await
        .unwrap();
    let patch = UserPatch {
        role: Some(Role::Admin),
        ..UserPatch::default()
    };
    let promoted = user::update_for_admin(&mut c, &verified, &m.id, &patch)
        .await
        .unwrap();
    assert_eq!(promoted.role, Role::Admin);
    assert_eq!(
        user::get_for_admin(&mut c, &verified, &m.id).await.unwrap(),
        promoted
    );
    let err = user::update_for_admin(&mut c, &verified, "fehlt", &patch).await;
    assert!(matches!(err, Err(DbError::NotFound)));
}
dual_test!(user_login_admin, login_and_admin);

/// An admin who is member of the account is invisible to the account scope: members can
/// neither read, change, lock nor delete them, and the scope can only create members.
async fn admin_invisible_to_members(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let system = AdminScope::system();
    let a = system.act_in(account::create(&mut c, &system, "A").await.unwrap().id);
    let new = NewUser {
        email: "admin@example.org".into(),
        name: "Admin".into(),
        role: Role::Admin,
        password_hash: None,
    };
    let admin = user::create_for_admin(&mut c, &system, Some(a.account_id()), &new)
        .await
        .unwrap();
    let created = user::create(&mut c, &a, &member("m@example.org"))
        .await
        .unwrap();
    assert_eq!(
        created.role,
        Role::Member,
        "Kunden-Scope legt nur Mitglieder an"
    );

    let ids: Vec<String> = user::list(&mut c, &a)
        .await
        .unwrap()
        .into_iter()
        .map(|u| u.id)
        .collect();
    assert_eq!(ids, [created.id]);
    let get = user::get(&mut c, &a, &admin.id).await;
    assert!(matches!(get, Err(DbError::NotFound)));
    let lock = MemberPatch {
        locked: Some(true),
        name: Some("X".into()),
    };
    let update = user::update(&mut c, &a, &admin.id, &lock).await;
    assert!(matches!(update, Err(DbError::NotFound)));
    let delete = user::delete(&mut c, &a, &admin.id).await;
    assert!(matches!(delete, Err(DbError::NotFound)));
    let unchanged = user::get_for_admin(&mut c, &system, &admin.id)
        .await
        .unwrap();
    assert_eq!(unchanged, admin, "Admin unverändert");
}
dual_test!(user_admin_hidden, admin_invisible_to_members);
