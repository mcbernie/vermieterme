//! The lock order of `repo::billing` under concurrency (two connections): creating a
//! period next to unit and property changes ends without a database error (#113), and
//! the locks behind decision 37 and the overlap check hold (#114). On SQLite the write
//! lock serializes everything; the MySQL runs are the real check.

// Test bodies are plain async fns run by `dual_test!` (see account_user_repo.rs).
#![allow(clippy::unwrap_used)]

mod billing_support;

use billing_support::{category, new_period, property, setup, unit_input};
use vermieterme_db::repo::billing::{PeriodPatch, Rejection, cost, period};
use vermieterme_db::repo::masterdata::{self, property as property_repo, unit};
use vermieterme_db::{Db, DbError, Dialect, dual_test, in_transaction_read_committed};

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

/// A first unit amount and moving its unit to another property at the same time: either
/// the move is refused (the amount came first) or the amount finds the unit gone from
/// the property (404). Never a moved unit with an amount (decision 37).
async fn parallel_unit_amount_and_move(db: &Db) {
    let s = setup(db, "A").await;
    let target = property(db, &s.scope).await;
    let notice = category(db, &s.scope, "Grundsteuer je Wohnung", "per_notice").await;
    let mut c1 = db.pool().acquire().await.unwrap();
    let mut c2 = db.pool().acquire().await.unwrap();
    for round in 0..ROUNDS {
        let uid = billing_support::unit(db, &s.scope, &s.property_id, 0).await;
        let key = (s.period.id.as_str(), notice.as_str(), uid.as_str());
        let moved = unit_input(&target, 0);
        let (saved, move_result) = tokio::join!(
            cost::set_unit_amount(&mut c1, &s.scope, key, Some(500), None),
            unit::update(&mut c2, &s.scope, &uid, &moved),
        );
        let move_result = move_result.unwrap();
        match saved {
            Ok(written) => {
                written.unwrap();
                assert_eq!(
                    move_result.unwrap_err(),
                    masterdata::Rejection::UnitInBillings,
                    "Runde {round}"
                );
            }
            Err(DbError::NotFound) => assert!(move_result.is_ok(), "Runde {round}"),
            Err(other) => panic!("Runde {round}: {other:?}"),
        }
    }
}
dual_test!(unit_amount_vs_move, parallel_unit_amount_and_move);

/// Moving a period into a free range while another period is created there: exactly one
/// is refused with an overlap. Only the property lock of both serializes them.
async fn parallel_period_update_and_create(db: &Db) {
    let s = setup(db, "A").await;
    let mut c1 = db.pool().acquire().await.unwrap();
    let mut c2 = db.pool().acquire().await.unwrap();
    for round in 0..ROUNDS {
        let year = 2040 + round;
        let first = new_period(
            &s.property_id,
            &format!("{year}-01-01"),
            &format!("{year}-03-31"),
        );
        let mut c = db.pool().acquire().await.unwrap();
        let existing = period::create(&mut c, &s.scope, &first)
            .await
            .unwrap()
            .unwrap();
        let shift = PeriodPatch {
            start_date: Some(format!("{year}-07-01")),
            end_date: Some(format!("{year}-12-31")),
            ..PeriodPatch::default()
        };
        let fresh = new_period(
            &s.property_id,
            &format!("{year}-09-01"),
            &format!("{year}-10-31"),
        );
        let (changed, made) = tokio::join!(
            period::update(&mut c1, &s.scope, &existing.id, &shift),
            period::create(&mut c2, &s.scope, &fresh),
        );
        let (changed, made) = (changed.unwrap(), made.unwrap());
        let refused = [&changed.as_ref().err(), &made.as_ref().err()];
        let overlaps = refused
            .iter()
            .filter(|r| matches!(r, Some(Rejection::Overlap { .. })))
            .count();
        assert_eq!(overlaps, 1, "Runde {round}: {changed:?} / {made:?}");
        assert!(changed.is_ok() != made.is_ok(), "Runde {round}");
    }
}
dual_test!(period_update_vs_create, parallel_period_update_and_create);

/// Two moves of one unit from the same stale read (P1): the first to P2 holds its locks
/// while the second, aimed at P3, reads P1 and waits. The second then finds the unit in
/// P2, a property it did not lock: it must refuse (409) instead of locking P2 after the
/// unit (lock order). On SQLite the write lock serializes them, the second just moves
/// on to P3.
async fn stale_unit_move_vs_move(db: &Db) {
    let s = setup(db, "A").await;
    let mut c1 = db.pool().acquire().await.unwrap();
    let mut c2 = db.pool().acquire().await.unwrap();
    for round in 0..ROUNDS {
        let (p2, p3) = (property(db, &s.scope).await, property(db, &s.scope).await);
        let uid = billing_support::unit(db, &s.scope, &s.property_id, 0).await;
        let (first, second) = (unit_input(&p2, 0), unit_input(&p3, 0));
        let (moved_tx, moved_rx) = tokio::sync::oneshot::channel::<()>();
        let (a, b) = tokio::join!(
            in_transaction_read_committed(&mut c1, async |tx| {
                unit::update(tx, &s.scope, &uid, &first).await?.unwrap();
                let _ = moved_tx.send(());
                // Keep the first move uncommitted while the second one starts.
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                Ok(())
            }),
            async {
                moved_rx.await.unwrap();
                unit::update(&mut c2, &s.scope, &uid, &second).await
            },
        );
        a.unwrap();
        let now = unit::get(&mut c1, &s.scope, &uid)
            .await
            .unwrap()
            .property_id;
        match (db.dialect(), b) {
            (Dialect::Sqlite, Ok(done)) => assert_eq!(done.unwrap().property_id, p3),
            (Dialect::MySql, Err(DbError::Conflict)) => assert_eq!(now, p2, "Runde {round}"),
            (_, other) => panic!("Runde {round}: {other:?}"),
        }
    }
}
dual_test!(stale_unit_move, stale_unit_move_vs_move);
