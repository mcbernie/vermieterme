//! Rate limits over the real router: parallel attempts, client address behind proxies.
//! The limiter lives in memory, so SQLite is enough here.

// Same convention as the other test binaries: helpers unwrap.
#![allow(clippy::unwrap_used)]

mod support;

use serde_json::json;
use support::{TestApp, seed_account, seed_user};
use vermieterme_db::repo::user::Role;
use vermieterme_db::testing::TestDb;

/// Parallel logins for one e-mail: at most the limit (5) reaches the password check,
/// the others are answered 429 at once.
#[tokio::test]
async fn parallel_logins_respect_the_limit() {
    let test_db = TestDb::sqlite().await;
    let db = test_db.db();
    let app = std::sync::Arc::new(TestApp::start(db).await);
    let account = seed_account(db, "Kunde").await;
    seed_user(db, Some(&account.id), "p@example.org", Role::Member).await;
    let wrong = json!({"email": "p@example.org", "password": "falsch-falsch"});
    let tasks: Vec<_> = (0..20)
        .map(|_| {
            let app = app.clone();
            let body = wrong.clone();
            tokio::spawn(async move { app.call("POST", "/api/auth/login", None, body).await })
        })
        .collect();
    let mut statuses = Vec::new();
    for task in tasks {
        statuses.push(task.await.unwrap().status);
    }
    let unauthorized = statuses.iter().filter(|s| **s == 401).count();
    let limited = statuses.iter().filter(|s| **s == 429).count();
    assert!(unauthorized <= 5, "{statuses:?}");
    assert_eq!(unauthorized + limited, 20, "{statuses:?}");
}
