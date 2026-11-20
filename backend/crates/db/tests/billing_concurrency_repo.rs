//! Billing under concurrency (two connections) and the copy of a period: parallel saves
//! of one cost position keep both changes (#25), a prepayment and a unit move serialize
//! on the unit (decision 37), deleting a category while a cost is saved gives a rule
//! refusal instead of a database error, overlapping creates leave one period.

// Test bodies are plain async fns run by `dual_test!` (see account_user_repo.rs).
#![allow(clippy::unwrap_used)]

mod billing_support;

use billing_support::{category, new_period, property, setup, unit_input};
use vermieterme_db::repo::billing::{
    CostPatch, NewPeriod, PrepaymentPatch, Rejection, cost, period, prepayment,
};
use vermieterme_db::repo::masterdata::{self, cost_category, unit};
use vermieterme_db::{Db, DbError, dual_test};
use vermieterme_domain::DistributionKey;

/// Rounds of each race: a missing lock shows only sometimes.
const ROUNDS: usize = 8;

/// Toggling `enabled` and saving the amount of the same position at the same time:
/// both changes survive, whichever runs first (the first save creates the row).
async fn parallel_cost_saves(db: &Db) {
    let s = setup(db, "A").await;
    let mut c1 = db.pool().acquire().await.unwrap();
    let mut c2 = db.pool().acquire().await.unwrap();
    for round in 0..ROUNDS {
        let cat = category(db, &s.scope, &format!("K{round}"), "mea").await;
        let pid = s.period.id.as_str();
        let toggle = CostPatch {
            enabled: Some(false),
            ..CostPatch::default()
        };
        let amount = CostPatch {
            total_amount_cents: Some(Some(4_200)),
            ..CostPatch::default()
        };
        let (x, y) = tokio::join!(
            cost::save(&mut c1, &s.scope, pid, &cat, &toggle),
            cost::save(&mut c2, &s.scope, pid, &cat, &amount),
        );
        x.unwrap().unwrap();
        y.unwrap().unwrap();
        let all = cost::list(&mut c1, &s.scope, pid).await.unwrap();
        let saved = all.iter().find(|c| c.cost_category_id == cat).unwrap();
        assert!(!saved.enabled, "Runde {round}");
        assert_eq!(saved.total_amount_cents, Some(4_200), "Runde {round}");
    }
}
dual_test!(cost_saves_parallel, parallel_cost_saves);

/// A first prepayment and moving its unit to another property at the same time: either
/// the move is refused (the prepayment came first) or the prepayment finds the unit gone
/// from the property (404). Never both, never a moved unit with a prepayment.
async fn parallel_prepayment_and_move(db: &Db) {
    let s = setup(db, "A").await;
    let target = property(db, &s.scope).await;
    let mut c1 = db.pool().acquire().await.unwrap();
    let mut c2 = db.pool().acquire().await.unwrap();
    for round in 0..ROUNDS {
        let uid = billing_support::unit(db, &s.scope, &s.property_id, 0).await;
        let patch = PrepaymentPatch {
            monthly_amount_cents: Some(10_000),
            ..PrepaymentPatch::default()
        };
        let moved = unit_input(&target, 0);
        let (saved, move_result) = tokio::join!(
            prepayment::save(&mut c1, &s.scope, &s.period.id, &uid, &patch),
            unit::update(&mut c2, &s.scope, &uid, &moved),
        );
        let move_result = move_result.unwrap();
        match saved {
            Ok(created) => {
                created.unwrap();
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
dual_test!(prepayment_vs_move, parallel_prepayment_and_move);

/// Deleting a category while a cost of it is saved: either the delete is refused
/// (category in use, 409) or the save finds the category gone (404).
async fn parallel_category_delete(db: &Db) {
    let s = setup(db, "A").await;
    let mut c1 = db.pool().acquire().await.unwrap();
    let mut c2 = db.pool().acquire().await.unwrap();
    for round in 0..ROUNDS {
        let cat = category(db, &s.scope, &format!("D{round}"), "mea").await;
        let patch = CostPatch {
            total_amount_cents: Some(Some(100)),
            ..CostPatch::default()
        };
        let (saved, deleted) = tokio::join!(
            cost::save(&mut c1, &s.scope, &s.period.id, &cat, &patch),
            cost_category::delete(&mut c2, &s.scope, &cat),
        );
        match saved {
            Ok(created) => {
                created.unwrap();
                let refusal = deleted.unwrap().unwrap_err();
                assert_eq!(
                    refusal,
                    masterdata::Rejection::CategoryInUse,
                    "Runde {round}"
                );
            }
            Err(DbError::NotFound) => deleted.unwrap().unwrap(),
            Err(other) => panic!("Runde {round}: {other:?}"),
        }
    }
}
dual_test!(category_delete_parallel, parallel_category_delete);

/// Two overlapping periods created at the same time: exactly one is refused.
async fn parallel_overlapping_creates(db: &Db) {
    let s = setup(db, "A").await;
    let mut c1 = db.pool().acquire().await.unwrap();
    let mut c2 = db.pool().acquire().await.unwrap();
    for round in 0..ROUNDS {
        let year = 2030 + round;
        let one = new_period(
            &s.property_id,
            &format!("{year}-01-01"),
            &format!("{year}-12-31"),
        );
        let two = new_period(
            &s.property_id,
            &format!("{year}-03-01"),
            &format!("{year}-09-30"),
        );
        let (x, y) = tokio::join!(
            period::create(&mut c1, &s.scope, &one),
            period::create(&mut c2, &s.scope, &two),
        );
        let (x, y) = (x.unwrap(), y.unwrap());
        assert!(x.is_ok() != y.is_ok(), "Runde {round}: {x:?} / {y:?}");
    }
}
dual_test!(overlapping_creates_parallel, parallel_overlapping_creates);

/// The copy takes over amounts, `enabled`, the key override, unit amounts and
/// prepayments, all unreviewed and at version 1; it records the source. A source of
/// another account is not found, one of another property refused.
async fn copy_period(db: &Db) {
    let s = setup(db, "A").await;
    let other = setup(db, "B").await;
    let mut c = db.pool().acquire().await.unwrap();
    let pid = s.period.id.as_str();
    let notice = category(db, &s.scope, "Wasser", "per_notice").await;
    let patch = CostPatch {
        total_amount_cents: Some(Some(90_000)),
        enabled: Some(false),
        override_key: Some(Some(DistributionKey::PerUnit)),
        ..CostPatch::default()
    };
    cost::save(&mut c, &s.scope, pid, &s.category_id, &patch)
        .await
        .unwrap()
        .unwrap();
    let key = (pid, notice.as_str(), s.units[1].as_str());
    cost::set_unit_amount(&mut c, &s.scope, key, Some(321), None)
        .await
        .unwrap()
        .unwrap();
    let amount = PrepaymentPatch {
        monthly_amount_cents: Some(20_000),
        ..PrepaymentPatch::default()
    };
    prepayment::save(&mut c, &s.scope, pid, &s.units[0], &amount)
        .await
        .unwrap()
        .unwrap();

    let input = NewPeriod {
        copy_from_id: Some(pid.into()),
        ..new_period(&s.property_id, "2025-01-01", "2025-12-31")
    };
    let copy = period::create(&mut c, &s.scope, &input).await;
    let copy = copy.unwrap().unwrap();
    assert_eq!(copy.copied_from_id.as_deref(), Some(pid));
    let costs = cost::list(&mut c, &s.scope, &copy.id).await.unwrap();
    assert_eq!(costs.len(), 2);
    let main = costs.iter().find(|c| c.cost_category_id == s.category_id);
    let main = main.unwrap();
    assert_eq!(main.total_amount_cents, Some(90_000));
    assert!(!main.enabled && !main.reviewed);
    assert_eq!((main.key(), main.version), (DistributionKey::PerUnit, 1));
    let water = costs.iter().find(|c| c.cost_category_id == notice).unwrap();
    assert_eq!(water.unit_amount(&s.units[1]), Some(321));
    let prepayments = prepayment::list(&mut c, &s.scope, &copy.id).await.unwrap();
    assert_eq!(prepayments.len(), 1);
    assert_eq!(prepayments[0].monthly_amount_cents, 20_000);
    assert!(!prepayments[0].reviewed);

    let foreign = NewPeriod {
        copy_from_id: Some(other.period.id.clone()),
        ..new_period(&s.property_id, "2026-01-01", "2026-12-31")
    };
    let refused = period::create(&mut c, &s.scope, &foreign).await;
    assert_eq!(refused.unwrap().unwrap_err(), Rejection::SourceNotFound);
    let elsewhere = property(db, &s.scope).await;
    billing_support::unit(db, &s.scope, &elsewhere, 10).await;
    let cross = NewPeriod {
        copy_from_id: Some(pid.into()),
        ..new_period(&elsewhere, "2026-01-01", "2026-12-31")
    };
    let refused = period::create(&mut c, &s.scope, &cross).await;
    assert_eq!(
        refused.unwrap().unwrap_err(),
        Rejection::SourceOtherProperty
    );
}
dual_test!(copy, copy_period);
