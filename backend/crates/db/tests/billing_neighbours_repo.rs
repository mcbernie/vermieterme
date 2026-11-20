//! Changes of periods of two neighbouring properties at the same time (#116): the
//! property `hi` follows `lo` in the index `(account_id, property_id, start_date)` of
//! `billing_period`, also across two accounts. A locking range read under
//! `REPEATABLE READ` locked the first period of `hi` behind the range of `lo` and
//! deadlocked with an insert in front of it; both changes must succeed. On SQLite the
//! write lock serializes everything; the MySQL runs are the real check.

// Test bodies are plain async fns run by `dual_test!` (see account_user_repo.rs).
#![allow(clippy::unwrap_used)]

mod billing_support;

use billing_support::{new_period, property, scope};
use vermieterme_db::repo::billing::{PeriodPatch, period};
use vermieterme_db::{AccountScope, Db, dual_test};

/// Rounds of each race (the deadlock showed in every round before #116).
const ROUNDS: usize = 30;

/// A property with one unit in `scope`.
async fn property_with_unit(db: &Db, scope: &AccountScope) -> String {
    let id = property(db, scope).await;
    billing_support::unit(db, scope, &id, 0).await;
    id
}

/// Two properties (each with its scope) in index order: `lo` before `hi`.
async fn neighbours<'a>(
    db: &Db,
    first: &'a AccountScope,
    second: &'a AccountScope,
) -> [(&'a AccountScope, String); 2] {
    let a = (first, property_with_unit(db, first).await);
    let b = (second, property_with_unit(db, second).await);
    let key = |(s, p): &(&AccountScope, String)| (s.account_id().to_owned(), p.clone());
    if key(&a) < key(&b) { [a, b] } else { [b, a] }
}

/// Creates a period of the year in the property.
async fn create_year(db: &Db, (scope, id): (&AccountScope, &str), year: u32) -> String {
    let mut c = db.pool().acquire().await.unwrap();
    let input = new_period(id, &format!("{year}-01-01"), &format!("{year}-12-31"));
    period::create(&mut c, scope, &input)
        .await
        .unwrap()
        .unwrap()
        .id
}

/// Per round: `hi` has a period 2025; `lo` creates 2030 (behind everything of `lo`)
/// while `hi` creates 2020 (in front of its 2025). With `shift`, `lo` instead moves its
/// period 2021 to 2030.
async fn race(db: &Db, first: &AccountScope, second: &AccountScope, shift: bool) {
    let mut c1 = db.pool().acquire().await.unwrap();
    let mut c2 = db.pool().acquire().await.unwrap();
    for round in 0..ROUNDS {
        let [(lo_scope, lo), (hi_scope, hi)] = neighbours(db, first, second).await;
        create_year(db, (hi_scope, &hi), 2025).await;
        let early = new_period(&hi, "2020-01-01", "2020-12-31");
        let (lo_result, hi_result) = if shift {
            let own = create_year(db, (lo_scope, &lo), 2021).await;
            let later = PeriodPatch {
                start_date: Some("2030-01-01".into()),
                end_date: Some("2030-12-31".into()),
                ..PeriodPatch::default()
            };
            tokio::join!(
                period::update(&mut c1, lo_scope, &own, &later),
                period::create(&mut c2, hi_scope, &early),
            )
        } else {
            let late = new_period(&lo, "2030-01-01", "2030-12-31");
            tokio::join!(
                period::create(&mut c1, lo_scope, &late),
                period::create(&mut c2, hi_scope, &early),
            )
        };
        assert!(
            matches!(lo_result, Ok(Ok(_))),
            "Runde {round}: {lo_result:?}"
        );
        assert!(
            matches!(hi_result, Ok(Ok(_))),
            "Runde {round}: {hi_result:?}"
        );
    }
}

/// Two creates in neighbouring properties of one account.
async fn create_vs_create(db: &Db) {
    let s = scope(db, "A").await;
    race(db, &s, &s, false).await;
}
dual_test!(neighbour_create_vs_create, create_vs_create);

/// Moving a period of `lo` while `hi` creates one.
async fn update_vs_create(db: &Db) {
    let s = scope(db, "A").await;
    race(db, &s, &s, true).await;
}
dual_test!(neighbour_update_vs_create, update_vs_create);

/// The same across two accounts (the index starts with `account_id`).
async fn across_accounts(db: &Db) {
    let a = scope(db, "A").await;
    let b = scope(db, "B").await;
    race(db, &a, &b, false).await;
    race(db, &a, &b, true).await;
}
dual_test!(neighbour_accounts, across_accounts);
