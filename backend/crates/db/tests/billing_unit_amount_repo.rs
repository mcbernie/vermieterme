//! Billing repositories: amounts of one unit in a cost position (#80, #115).

// Test bodies are plain async fns run by `dual_test!` (see account_user_repo.rs).
#![allow(clippy::unwrap_used)]

mod billing_support;

use billing_support::{category, property, setup, unit, unit_input};
use vermieterme_db::repo::billing::{Cost, CostPatch, Rejection, UnitAmountSaved, cost};
use vermieterme_db::repo::masterdata::cost_category::{self, CostCategoryInput};
use vermieterme_db::repo::masterdata::{self, unit as unit_repo};
use vermieterme_db::{Db, DbError, dual_test};
use vermieterme_domain::DistributionKey;

/// Unit amounts: set, change (unreviewed, version), clear; a unit of another property
/// or account is not found; a new position is created on the way, but not for clearing;
/// a key in effect that computes shares refuses amounts (#115).
async fn unit_amounts(db: &Db) {
    let s = setup(db, "A").await;
    let mut c = db.pool().acquire().await.unwrap();
    let cat = category(db, &s.scope, "Wasser", "per_notice").await;
    let key = (s.period.id.as_str(), cat.as_str(), s.units[0].as_str());

    let nothing = cost::set_unit_amount(&mut c, &s.scope, key, None, None).await;
    assert_eq!(nothing.unwrap().unwrap(), UnitAmountSaved::NoPosition);
    let costs = cost::list(&mut c, &s.scope, &s.period.id).await.unwrap();
    assert!(costs.is_empty());

    let set = cost::set_unit_amount(&mut c, &s.scope, key, Some(500), None).await;
    let set = written(set.unwrap().unwrap());
    assert_eq!(set.unit_amount(&s.units[0]), Some(500));
    assert_eq!(
        (set.version, set.reviewed, set.total_amount_cents),
        (1, true, None)
    );
    let same = cost::set_unit_amount(&mut c, &s.scope, key, Some(500), Some(1)).await;
    let same = same.unwrap().unwrap();
    assert!(matches!(&same, UnitAmountSaved::Unchanged(c) if c.version == 1));
    let changed = cost::set_unit_amount(&mut c, &s.scope, key, Some(700), Some(1)).await;
    let changed = written(changed.unwrap().unwrap());
    assert_eq!((changed.version, changed.reviewed), (2, false));
    let stale = cost::set_unit_amount(&mut c, &s.scope, key, Some(1), Some(1)).await;
    assert!(matches!(stale.unwrap(), Err(Rejection::CostVersion(_))));
    let cleared = cost::set_unit_amount(&mut c, &s.scope, key, None, None).await;
    assert!(written(cleared.unwrap().unwrap()).unit_amounts.is_empty());

    let elsewhere = property(db, &s.scope).await;
    let stranger = unit(db, &s.scope, &elsewhere, 10).await;
    let key = (s.period.id.as_str(), cat.as_str(), stranger.as_str());
    let refused = cost::set_unit_amount(&mut c, &s.scope, key, Some(1), None).await;
    assert!(matches!(refused, Err(DbError::NotFound)));

    // MEA: refused for a new and an existing position; an override to a key by notice
    // allows it.
    let mea = (
        s.period.id.as_str(),
        s.category_id.as_str(),
        s.units[0].as_str(),
    );
    let refused = cost::set_unit_amount(&mut c, &s.scope, mea, Some(1), None).await;
    assert_eq!(
        refused.unwrap().unwrap_err(),
        Rejection::UnitAmountKey(DistributionKey::Mea)
    );
    let total = CostPatch {
        total_amount_cents: Some(Some(1_000)),
        ..CostPatch::default()
    };
    let pid = s.period.id.as_str();
    cost::save(&mut c, &s.scope, pid, &s.category_id, &total)
        .await
        .unwrap()
        .unwrap();
    let refused = cost::set_unit_amount(&mut c, &s.scope, mea, Some(1), None).await;
    assert!(matches!(refused.unwrap(), Err(Rejection::UnitAmountKey(_))));
    let by_notice = CostPatch {
        override_key: Some(Some(DistributionKey::PerAttachment)),
        ..CostPatch::default()
    };
    cost::save(&mut c, &s.scope, pid, &s.category_id, &by_notice)
        .await
        .unwrap()
        .unwrap();
    let set = cost::set_unit_amount(&mut c, &s.scope, mea, Some(1), None).await;
    assert_eq!(
        written(set.unwrap().unwrap()).unit_amount(&s.units[0]),
        Some(1)
    );
}

/// The position of a write that must have written.
fn written(saved: UnitAmountSaved) -> Cost {
    match saved {
        UnitAmountSaved::Written(cost) => cost,
        other => panic!("nicht geschrieben: {other:?}"),
    }
}
dual_test!(unit_amount_rules, unit_amounts);

/// Unit amounts block moving their unit only while the key in effect uses them (#117):
/// after an override to `mea` the amount no longer counts, and the move deletes it.
async fn ignored_amounts_do_not_block(db: &Db) {
    let s = setup(db, "A").await;
    let mut c = db.pool().acquire().await.unwrap();
    let cat = category(db, &s.scope, "Wasser", "per_notice").await;
    let pid = s.period.id.as_str();
    let key = (pid, cat.as_str(), s.units[0].as_str());
    cost::set_unit_amount(&mut c, &s.scope, key, Some(500), None)
        .await
        .unwrap()
        .unwrap();
    let target = property(db, &s.scope).await;
    let moved = unit_input(&target, 0);
    let refused = unit_repo::update(&mut c, &s.scope, &s.units[0], &moved).await;
    assert_eq!(
        refused.unwrap().unwrap_err(),
        masterdata::Rejection::UnitInBillings
    );

    let to_mea = CostPatch {
        override_key: Some(Some(DistributionKey::Mea)),
        ..CostPatch::default()
    };
    cost::save(&mut c, &s.scope, pid, &cat, &to_mea)
        .await
        .unwrap()
        .unwrap();
    let done = unit_repo::update(&mut c, &s.scope, &s.units[0], &moved).await;
    assert_eq!(done.unwrap().unwrap().property_id, target);
    let left = cost::list(&mut c, &s.scope, pid).await.unwrap();
    assert_eq!(
        left[0].unit_amount(&s.units[0]),
        None,
        "mit umgehängt gelöscht"
    );
}
dual_test!(ignored_unit_amounts, ignored_amounts_do_not_block);

/// Changing the key of a category back to one that uses unit amounts while a unit with
/// an ignored amount of it moves: either the move comes first (amount deleted, moved) or
/// the change does (move refused, amount kept). Never a moved unit whose amount counts.
async fn key_change_vs_move(db: &Db) {
    let s = setup(db, "A").await;
    let target = property(db, &s.scope).await;
    let mut c1 = db.pool().acquire().await.unwrap();
    let mut c2 = db.pool().acquire().await.unwrap();
    let pid = s.period.id.as_str();
    for round in 0..8 {
        let name = format!("K{round}");
        let cat = category(db, &s.scope, &name, "per_notice").await;
        let uid = unit(db, &s.scope, &s.property_id, 0).await;
        cost::set_unit_amount(
            &mut c1,
            &s.scope,
            (pid, cat.as_str(), uid.as_str()),
            Some(500),
            None,
        )
        .await
        .unwrap()
        .unwrap();
        let with_key = |key: &str| CostCategoryInput {
            name: name.clone(),
            distribution_key: key.into(),
            sort_order: 0,
        };
        cost_category::update(&mut c1, &s.scope, &cat, &with_key("mea"))
            .await
            .unwrap();
        let moved = unit_input(&target, 0);
        let back = with_key("per_notice");
        let (changed, move_result) = tokio::join!(
            cost_category::update(&mut c1, &s.scope, &cat, &back),
            unit_repo::update(&mut c2, &s.scope, &uid, &moved),
        );
        changed.unwrap();
        let costs = cost::list(&mut c1, &s.scope, pid).await.unwrap();
        let amount = costs
            .iter()
            .find(|c| c.cost_category_id == cat)
            .and_then(|c| c.unit_amount(&uid));
        match move_result.unwrap() {
            Ok(_) => assert_eq!(amount, None, "Runde {round}"),
            Err(refusal) => {
                assert_eq!(
                    refusal,
                    masterdata::Rejection::UnitInBillings,
                    "Runde {round}"
                );
                assert_eq!(amount, Some(500), "Runde {round}");
            }
        }
    }
}
dual_test!(unit_amount_key_change_vs_move, key_change_vs_move);
