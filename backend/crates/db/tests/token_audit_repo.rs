//! Repositories for one-time links and the audit log.

// Test bodies are plain async fns run by `dual_test!`, so clippy's in-test allowance
// does not reach them.
#![allow(clippy::unwrap_used)]
// Tests build scopes directly (no session layer here); see vermieterme_db::scope.
#![allow(clippy::disallowed_methods)]

#[path = "support/fixtures.rs"]
mod fixtures;

use fixtures::{HOUR, NOW, setup};
use std::collections::HashSet;

use vermieterme_db::repo::audit::{self, AuditCursor, AuditFilter, NewAuditEntry};
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

/// Two connections redeem the same link at the same time: exactly one wins. SQLite
/// serialises the two writers (busy timeout), MariaDB row-locks the token row; in both
/// the conditional `UPDATE` of the loser sees `used_at` set and changes nothing.
async fn concurrent_consume(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let (_, uid) = setup(&mut c, "m@example.org", Role::Member).await;
    let mut a = db.pool().acquire().await.unwrap();
    let mut b = db.pool().acquire().await.unwrap();
    // Several rounds, so the two statements really overlap at least sometimes.
    for _ in 0..10 {
        let token = SecretToken::generate();
        user_token::issue(&mut c, &uid, TokenPurpose::Reset, &token, NOW, NOW + HOUR)
            .await
            .unwrap();
        let (ra, rb) = tokio::join!(
            user_token::consume(&mut a, TokenPurpose::Reset, &token, NOW),
            user_token::consume(&mut b, TokenPurpose::Reset, &token, NOW),
        );
        let winners: Vec<String> = [ra.unwrap(), rb.unwrap()].into_iter().flatten().collect();
        assert_eq!(winners, std::slice::from_ref(&uid), "genau eine Einlösung");
    }
}
dual_test!(user_token_concurrent, concurrent_consume);

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
    let page = audit::list(&mut c, &admin, &AuditFilter::default())
        .await
        .unwrap();
    assert_eq!(page.next, None, "nur eine Seite");
    let all = page.items;
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
    assert_eq!(
        audit::list(&mut c, &admin, &filter)
            .await
            .unwrap()
            .items
            .len(),
        2
    );
    let filter = AuditFilter {
        account_id: None,
        before: Some(AuditCursor {
            at: 0,
            id: String::new(),
        }),
    };
    assert!(
        audit::list(&mut c, &admin, &filter)
            .await
            .unwrap()
            .items
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
    assert_eq!(page.items.len(), usize::try_from(audit::MAX_PAGE).unwrap());
    assert_eq!(page.next, page.items.last().map(AuditCursor::after));
}
dual_test!(audit_entries, audit_log);

/// Inserts an audit entry with a fixed id at [`NOW`].
async fn insert_at_now(c: &mut sqlx::AnyConnection, id: &str) {
    sqlx::query(
        "INSERT INTO audit_log (id, at, user_id, account_id, action, entity, entity_id) \
         VALUES (?, ?, NULL, NULL, 'x', 'y', NULL)",
    )
    .bind(id)
    .bind(NOW)
    .execute(&mut *c)
    .await
    .unwrap();
}

/// Entries sharing one millisecond across a page boundary are all returned exactly once:
/// the cursor `(at, id)` breaks ties by id. Exactly one full page has no next cursor.
async fn audit_paging_same_millisecond(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let admin = AdminScope::system();
    let total = audit::MAX_PAGE + 1;
    // Inserted out of id order, all at the same instant.
    let ids: Vec<String> = (0..total)
        .map(|i| format!("e{:03}", (i * 37) % total))
        .collect();
    for id in &ids[..ids.len() - 1] {
        insert_at_now(&mut c, id).await;
    }
    let full = audit::list(&mut c, &admin, &AuditFilter::default())
        .await
        .unwrap();
    assert_eq!(full.items.len(), usize::try_from(audit::MAX_PAGE).unwrap());
    assert_eq!(full.next, None, "genau eine volle Seite: kein Cursor");

    insert_at_now(&mut c, &ids[ids.len() - 1]).await;
    let first = audit::list(&mut c, &admin, &AuditFilter::default())
        .await
        .unwrap();
    assert_eq!(first.items.len(), usize::try_from(audit::MAX_PAGE).unwrap());
    assert_eq!(first.next, first.items.last().map(AuditCursor::after));
    let filter = AuditFilter {
        account_id: None,
        before: first.next.clone(),
    };
    let second = audit::list(&mut c, &admin, &filter).await.unwrap();
    assert_eq!(second.items.len(), 1, "Rest derselben Millisekunde");
    assert_eq!(second.next, None);

    let seen: Vec<&str> = first
        .items
        .iter()
        .chain(&second.items)
        .map(|e| e.id.as_str())
        .collect();
    let unique: HashSet<&str> = seen.iter().copied().collect();
    assert_eq!(unique.len(), seen.len(), "keine Dubletten");
    assert_eq!(unique.len(), ids.len(), "alle Einträge");
    assert!(
        seen.windows(2).all(|w| w[0] > w[1]),
        "absteigend nach id innerhalb der Millisekunde"
    );
}
dual_test!(audit_paging, audit_paging_same_millisecond);
