//! Repositories for one-time links and the audit log.

// Test bodies are plain async fns run by `dual_test!`, so clippy's in-test allowance
// does not reach them.
#![allow(clippy::unwrap_used)]
// Tests build scopes directly (no session layer here); see vermieterme_db::scope.
#![allow(clippy::disallowed_methods)]

#[path = "support/fixtures.rs"]
mod fixtures;

use fixtures::{HOUR, NOW, setup};
use vermieterme_db::repo::audit::{self, AuditFilter, NewAuditEntry};
use vermieterme_db::repo::user::Role;
use vermieterme_db::repo::user_token::{self, TokenPurpose};
use vermieterme_db::{AdminScope, Db, SecretToken, dual_test};

/// One-time links work once, only before expiry, only for their purpose, newest only.
async fn one_time_links(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let (_, uid) = setup(&mut c, "m@example.org", Role::Member).await;
    let first = SecretToken::generate();
    user_token::issue(&mut c, &uid, TokenPurpose::Invite, &first, NOW, NOW + HOUR)
        .await
        .unwrap();
    let second = SecretToken::generate();
    user_token::issue(&mut c, &uid, TokenPurpose::Invite, &second, NOW, NOW + HOUR)
        .await
        .unwrap();

    assert_eq!(
        user_token::consume(&mut c, TokenPurpose::Invite, &first, NOW)
            .await
            .unwrap(),
        None,
        "ersetzt"
    );
    assert_eq!(
        user_token::consume(&mut c, TokenPurpose::Reset, &second, NOW)
            .await
            .unwrap(),
        None,
        "falscher Zweck"
    );
    assert_eq!(
        user_token::consume(&mut c, TokenPurpose::Invite, &second, NOW + HOUR)
            .await
            .unwrap(),
        None,
        "abgelaufen"
    );
    let used = user_token::consume(&mut c, TokenPurpose::Invite, &second, NOW)
        .await
        .unwrap();
    assert_eq!(used.as_deref(), Some(uid.as_str()));
    assert_eq!(
        user_token::consume(&mut c, TokenPurpose::Invite, &second, NOW)
            .await
            .unwrap(),
        None,
        "einmalig"
    );
}
dual_test!(user_token_once, one_time_links);

/// Entries are listed newest first, filterable, limited, with the actor's e-mail.
async fn audit_log(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let (scope, uid) = setup(&mut c, "admin@example.org", Role::Admin).await;
    let admin = AdminScope::verified_admin(&uid);
    for i in 0..3 {
        let entry = NewAuditEntry {
            user_id: Some(&uid),
            account_id: if i == 1 {
                None
            } else {
                Some(scope.account_id())
            },
            action: "account.switch",
            entity: "account",
            entity_id: Some(scope.account_id()),
        };
        audit::record(&mut c, &entry).await.unwrap();
    }
    let all = audit::list(&mut c, &admin, &AuditFilter::default())
        .await
        .unwrap();
    assert_eq!(all.len(), 3);
    assert!(
        all.windows(2)
            .all(|w| (w[0].at, &w[0].id) >= (w[1].at, &w[1].id)),
        "neueste zuerst"
    );
    assert_eq!(all[0].user_email.as_deref(), Some("admin@example.org"));
    assert_eq!(all[0].action, "account.switch");

    let filter = AuditFilter {
        account_id: Some(scope.account_id().into()),
        before: None,
    };
    assert_eq!(audit::list(&mut c, &admin, &filter).await.unwrap().len(), 2);
    let filter = AuditFilter {
        account_id: None,
        before: Some(0),
    };
    assert!(
        audit::list(&mut c, &admin, &filter)
            .await
            .unwrap()
            .is_empty()
    );

    for _ in 0..audit::MAX_PAGE {
        let entry = NewAuditEntry {
            user_id: None,
            account_id: None,
            action: "x",
            entity: "y",
            entity_id: None,
        };
        audit::record(&mut c, &entry).await.unwrap();
    }
    let page = audit::list(&mut c, &admin, &AuditFilter::default())
        .await
        .unwrap();
    assert_eq!(page.len(), usize::try_from(audit::MAX_PAGE).unwrap());
}
dual_test!(audit_entries, audit_log);
