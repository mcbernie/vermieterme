//! Repository for web sessions.

// Test bodies are plain async fns run by `dual_test!`, so clippy's in-test allowance
// does not reach them.
#![allow(clippy::unwrap_used)]

#[path = "support/fixtures.rs"]
mod fixtures;

use fixtures::{HOUR, NOW, setup};
use sqlx::Row;
use vermieterme_db::repo::account::{self, AccountPatch};
use vermieterme_db::repo::session;
use vermieterme_db::repo::user::{self, NewUser, Role, UserPatch};
use vermieterme_db::{AdminScope, Db, SecretToken, dual_test};

/// Only the hash is stored; expiry, idle timeout and locks end the session.
async fn session_validity(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let (scope, uid) = setup(&mut c, "m@example.org", Role::Member).await;
    let token = SecretToken::generate();
    let s = session::create(&mut c, &uid, &token, NOW, NOW + 24 * HOUR)
        .await
        .unwrap();

    let stored: String = sqlx::query("SELECT token_hash FROM web_session")
        .fetch_one(&mut *c)
        .await
        .unwrap()
        .get("token_hash");
    assert_eq!(stored, token.hash().as_str());
    assert_ne!(stored, token.expose(), "nie der Klartext");

    let found = session::find_valid(&mut c, &token, NOW + 1, NOW - HOUR)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (found.session.id.as_str(), found.user.id.as_str()),
        (s.id.as_str(), uid.as_str())
    );
    let other = SecretToken::generate();
    assert!(
        session::find_valid(&mut c, &other, NOW, NOW - HOUR)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        session::find_valid(&mut c, &token, NOW + 24 * HOUR, 0)
            .await
            .unwrap()
            .is_none(),
        "abgelaufen"
    );
    assert!(
        session::find_valid(&mut c, &token, NOW, NOW)
            .await
            .unwrap()
            .is_none(),
        "Leerlauf"
    );
    session::touch(&mut c, &s.id, NOW + 2 * HOUR).await.unwrap();
    assert!(
        session::find_valid(&mut c, &token, NOW + 2 * HOUR, NOW + HOUR)
            .await
            .unwrap()
            .is_some()
    );

    let admin = AdminScope::system();
    let lock = UserPatch {
        locked: Some(true),
        ..UserPatch::default()
    };
    user::update(&mut c, &scope, &uid, &lock).await.unwrap();
    assert!(
        session::find_valid(&mut c, &token, NOW, 0)
            .await
            .unwrap()
            .is_none(),
        "Benutzer gesperrt"
    );
    let unlock = UserPatch {
        locked: Some(false),
        ..UserPatch::default()
    };
    user::update(&mut c, &scope, &uid, &unlock).await.unwrap();
    let lock = AccountPatch {
        locked: Some(true),
        name: None,
    };
    account::update(&mut c, &admin, scope.account_id(), &lock)
        .await
        .unwrap();
    assert!(
        session::find_valid(&mut c, &token, NOW, 0)
            .await
            .unwrap()
            .is_none(),
        "Kunde gesperrt"
    );
}
dual_test!(session_valid, session_validity);

/// Ending sessions: per user (keeping one), per account, expired ones, logout.
async fn session_ending(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let (scope, uid) = setup(&mut c, "m@example.org", Role::Member).await;
    let admin = AdminScope::system();
    let tokens: Vec<SecretToken> = (0..3).map(|_| SecretToken::generate()).collect();
    let mut ids = Vec::new();
    for t in &tokens {
        ids.push(
            session::create(&mut c, &uid, t, NOW, NOW + HOUR)
                .await
                .unwrap()
                .id,
        );
    }
    assert_eq!(
        session::delete_for_user(&mut c, &uid, Some(&ids[0]))
            .await
            .unwrap(),
        2
    );
    assert!(
        session::find_valid(&mut c, &tokens[0], NOW, 0)
            .await
            .unwrap()
            .is_some()
    );

    // Admin switched into the account keeps the session, loses the switch.
    let admin_new = NewUser {
        email: "admin@example.org".into(),
        name: "A".into(),
        role: Role::Admin,
        password_hash: None,
    };
    let root = user::create_admin(&mut c, &admin, &admin_new)
        .await
        .unwrap();
    let admin_token = SecretToken::generate();
    let admin_session = session::create(&mut c, &root.id, &admin_token, NOW, NOW + HOUR)
        .await
        .unwrap();
    session::set_acting_account(&mut c, &admin, &admin_session.id, Some(scope.account_id()))
        .await
        .unwrap();
    let acting = session::find_valid(&mut c, &admin_token, NOW, 0)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        acting.session.acting_account_id.as_deref(),
        Some(scope.account_id())
    );

    assert_eq!(
        session::delete_for_account(&mut c, &admin, scope.account_id())
            .await
            .unwrap(),
        1
    );
    assert!(
        session::find_valid(&mut c, &tokens[0], NOW, 0)
            .await
            .unwrap()
            .is_none()
    );
    let back = session::find_valid(&mut c, &admin_token, NOW, 0)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(back.session.acting_account_id, None);

    assert_eq!(
        session::delete_expired(&mut c, NOW + HOUR, 0)
            .await
            .unwrap(),
        1
    );
    let t = SecretToken::generate();
    let s = session::create(&mut c, &root.id, &t, NOW, NOW + HOUR)
        .await
        .unwrap();
    session::delete(&mut c, &s.id).await.unwrap();
    assert!(
        session::find_valid(&mut c, &t, NOW, 0)
            .await
            .unwrap()
            .is_none()
    );
}
dual_test!(session_end, session_ending);
