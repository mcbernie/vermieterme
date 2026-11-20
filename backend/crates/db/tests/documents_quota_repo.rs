//! Storage quota of an account (decision 44, #124): the sum of its documents may not
//! exceed the limit, other accounts are not affected, and two uploads of one account into
//! different properties (different property locks) cannot both pass the check. On SQLite
//! the write lock serializes everything; the MySQL runs are the real check.

// Test bodies are plain async fns run by `dual_test!` (see account_user_repo.rs).
#![allow(clippy::unwrap_used)]
// Tests build scopes directly (no session layer here); see vermieterme_db::scope.
#![allow(clippy::disallowed_methods)]

mod billing_support;

use billing_support::{new_period, property, scope, unit};
use vermieterme_db::repo::billing::period;
use vermieterme_db::repo::documents::{self, NewDocument, QuotaExceeded, used_bytes};
use vermieterme_db::{AccountScope, Db, dual_test};

/// Rounds of the race: a missing lock shows only sometimes.
const ROUNDS: usize = 8;

/// A document row of `size` bytes, optionally in a billing period.
fn doc(file: &str, size: i64, period: Option<&str>) -> NewDocument {
    NewDocument {
        file_name: file.into(),
        original_name: "a.pdf".into(),
        mime_type: "application/pdf".into(),
        size_bytes: size,
        category: "other".into(),
        billing_period_id: period.map(Into::into),
        tenant_id: None,
    }
}

/// A billing period of a new property (with one unit) of the account; returns its id.
async fn period_of_new_property(db: &Db, scope: &AccountScope) -> String {
    let pid = property(db, scope).await;
    unit(db, scope, &pid, 10).await;
    let mut c = db.pool().acquire().await.unwrap();
    let input = new_period(&pid, "2024-01-01", "2024-12-31");
    period::create(&mut c, scope, &input)
        .await
        .unwrap()
        .unwrap()
        .id
}

/// Documents up to the quota fit, one byte more does not; other accounts count apart;
/// deleting frees the space.
async fn quota_per_account(db: &Db) {
    let (a, b) = (scope(db, "A").await, scope(db, "B").await);
    let mut c = db.pool().acquire().await.unwrap();
    let first = documents::create_within_quota(&mut c, &a, &doc("a1", 6, None), 10)
        .await
        .unwrap()
        .unwrap();
    let refused = documents::create_within_quota(&mut c, &a, &doc("a2", 5, None), 10).await;
    assert_eq!(refused.unwrap(), Err(QuotaExceeded));
    documents::create_within_quota(&mut c, &a, &doc("a3", 4, None), 10)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(used_bytes(&mut c, &a).await.unwrap(), 10);
    documents::create_within_quota(&mut c, &b, &doc("b1", 10, None), 10)
        .await
        .unwrap()
        .unwrap();
    documents::delete(&mut c, &a, &first.id).await.unwrap();
    assert_eq!(used_bytes(&mut c, &a).await.unwrap(), 4);
    documents::create_within_quota(&mut c, &a, &doc("a4", 6, None), 10)
        .await
        .unwrap()
        .unwrap();
}
dual_test!(quota_per_account_cases, quota_per_account);

/// Two uploads of one account into different properties at the same time, room for only
/// one: exactly one is stored, the other is refused (never both, never a deadlock).
async fn parallel_uploads(db: &Db) {
    let mut c1 = db.pool().acquire().await.unwrap();
    let mut c2 = db.pool().acquire().await.unwrap();
    for round in 0..ROUNDS {
        let s = scope(db, &format!("Q{round}")).await;
        let (p1, p2) = (
            period_of_new_property(db, &s).await,
            period_of_new_property(db, &s).await,
        );
        let (one, two) = (
            doc(&format!("x{round}"), 6, Some(&p1)),
            doc(&format!("y{round}"), 6, Some(&p2)),
        );
        let (a, b) = tokio::join!(
            documents::create_within_quota(&mut c1, &s, &one, 10),
            documents::create_within_quota(&mut c2, &s, &two, 10),
        );
        let (a, b) = (a.unwrap(), b.unwrap());
        assert_eq!(
            usize::from(a.is_ok()) + usize::from(b.is_ok()),
            1,
            "Runde {round}: genau ein Upload passt"
        );
        let mut c = db.pool().acquire().await.unwrap();
        assert_eq!(used_bytes(&mut c, &s).await.unwrap(), 6);
    }
}
dual_test!(parallel_uploads_cases, parallel_uploads);
