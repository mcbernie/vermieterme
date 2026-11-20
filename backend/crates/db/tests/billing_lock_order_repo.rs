//! The lock order of `repo::billing` under concurrency (two connections): creating a
//! period next to unit and property changes ends without a database error (#113). On
//! SQLite the write lock serializes everything; the MySQL runs are the real check.

// Test bodies are plain async fns run by `dual_test!` (see account_user_repo.rs).
#![allow(clippy::unwrap_used)]

mod billing_support;

use billing_support::{new_period, property, setup, unit_input};
use vermieterme_db::repo::billing::period;
use vermieterme_db::repo::masterdata::{property as property_repo, unit};
use vermieterme_db::{Db, DbError, dual_test};

/// Rounds of each race: a wrong lock order shows only sometimes.
const ROUNDS: i32 = 8;

/// A period created while a unit is created in, and another one moved into, the same
/// property: all three succeed (before #113 MySQL aborted one with a deadlock).
async fn period_create_vs_unit_changes(db: &Db) {
    let s = setup(db, "A").await;
    let other = property(db, &s.scope).await;
    let mut c1 = db.pool().acquire().await.unwrap();
    let mut c2 = db.pool().acquire().await.unwrap();
    let mut c3 = db.pool().acquire().await.unwrap();
    for round in 0..ROUNDS {
        let year = 2030 + round;
        let input = new_period(
            &s.property_id,
            &format!("{year}-01-01"),
            &format!("{year}-12-31"),
        );
        let mover = billing_support::unit(db, &s.scope, &other, 0).await;
        let moved = unit_input(&s.property_id, 0);
        let added = unit_input(&s.property_id, 0);
        let (made, added, moved) = tokio::join!(
            period::create(&mut c1, &s.scope, &input),
            unit::create(&mut c2, &s.scope, &added),
            unit::update(&mut c3, &s.scope, &mover, &moved),
        );
        made.unwrap().unwrap();
        added.unwrap().unwrap();
        moved.unwrap().unwrap();
    }
}
dual_test!(period_create_vs_units, period_create_vs_unit_changes);

/// Deleting a property while a period of it is created: either the period comes first
/// and goes with the property, or the property is gone (404). Never a database error.
async fn period_create_vs_property_delete(db: &Db) {
    let s = setup(db, "A").await;
    let mut c1 = db.pool().acquire().await.unwrap();
    let mut c2 = db.pool().acquire().await.unwrap();
    for round in 0..ROUNDS {
        let doomed = property(db, &s.scope).await;
        billing_support::unit(db, &s.scope, &doomed, 0).await;
        let input = new_period(&doomed, "2024-01-01", "2024-12-31");
        let (made, deleted) = tokio::join!(
            period::create(&mut c1, &s.scope, &input),
            property_repo::delete(&mut c2, &s.scope, &doomed),
        );
        deleted.unwrap();
        match made {
            Ok(created) => {
                let created = created.unwrap();
                let gone = period::get(&mut c1, &s.scope, &created.id).await;
                assert!(matches!(gone, Err(DbError::NotFound)), "Runde {round}");
            }
            Err(DbError::NotFound) => {}
            Err(other) => panic!("Runde {round}: {other:?}"),
        }
    }
}
dual_test!(period_create_vs_delete, period_create_vs_property_delete);
