//! Billing repositories: periods (overlap, copy), cost positions (partial update, review
//! rule, version) and prepayments, with foreign ids as not found.

// Test bodies are plain async fns run by `dual_test!` (see account_user_repo.rs).
#![allow(clippy::unwrap_used)]

mod billing_support;

use billing_support::{new_period, property, setup};
use vermieterme_db::repo::billing::{
    CostPatch, PeriodPatch, PrepaymentPatch, Rejection, Saved, cost, period, prepayment, summary,
};
use vermieterme_db::{Db, DbError, dual_test};
use vermieterme_domain::DistributionKey;

/// Periods: list with counts, overlap (a shared boundary day too), no units, end before start,
/// update and delete; another account sees nothing.
async fn periods(db: &Db) {
    let s = setup(db, "A").await;
    let other = setup(db, "B").await;
    let mut c = db.pool().acquire().await.unwrap();
    let id = &s.period.id;

    let overlapping = new_period(&s.property_id, "2024-06-01", "2025-05-31");
    let refused = period::create(&mut c, &s.scope, &overlapping).await;
    assert!(matches!(
        refused.unwrap().unwrap_err(),
        Rejection::Overlap { start, end } if start == "2024-01-01" && end == "2024-12-31"
    ));
    // Decision 40: a shared boundary day overlaps, the next day is free.
    let boundary = new_period(&s.property_id, "2024-12-31", "2025-12-31");
    let refused = period::create(&mut c, &s.scope, &boundary).await;
    assert!(matches!(
        refused.unwrap().unwrap_err(),
        Rejection::Overlap { start, .. } if start == "2024-01-01"
    ));
    let adjacent = new_period(&s.property_id, "2025-01-01", "2025-12-31");
    let next = period::create(&mut c, &s.scope, &adjacent).await;
    let next = next.unwrap().unwrap();

    let empty = property(db, &s.scope).await;
    let no_units = new_period(&empty, "2024-01-01", "2024-12-31");
    let refused = period::create(&mut c, &s.scope, &no_units).await;
    assert_eq!(refused.unwrap().unwrap_err(), Rejection::NoUnits);
    let foreign = new_period(&other.property_id, "2030-01-01", "2030-12-31");
    let missing = period::create(&mut c, &s.scope, &foreign).await;
    assert!(matches!(missing, Err(DbError::NotFound)));

    let list = summary::list(&mut c, &s.scope, Some(&s.property_id)).await;
    let ids: Vec<_> = list.unwrap().into_iter().map(|p| p.period.id).collect();
    assert_eq!(ids, vec![next.id.clone(), id.clone()]);

    let backwards = PeriodPatch {
        end_date: Some("2023-12-31".into()),
        ..PeriodPatch::default()
    };
    let refused = period::update(&mut c, &s.scope, id, &backwards).await;
    assert_eq!(refused.unwrap().unwrap_err(), Rejection::EndNotAfterStart);
    let into_next = PeriodPatch {
        end_date: Some("2025-02-01".into()),
        ..PeriodPatch::default()
    };
    let refused = period::update(&mut c, &s.scope, id, &into_next).await;
    assert!(matches!(refused.unwrap(), Err(Rejection::Overlap { .. })));
    let onto_boundary = PeriodPatch {
        end_date: Some("2025-01-01".into()),
        ..PeriodPatch::default()
    };
    let refused = period::update(&mut c, &s.scope, id, &onto_boundary).await;
    assert!(matches!(refused.unwrap(), Err(Rejection::Overlap { .. })));
    let dates = PeriodPatch {
        billing_date: Some(Some("2025-03-01".into())),
        sent_date: Some(Some("2025-03-02".into())),
        ..PeriodPatch::default()
    };
    let changed = period::update(&mut c, &s.scope, id, &dates).await;
    let changed = changed.unwrap().unwrap();
    assert_eq!(changed.sent_date.as_deref(), Some("2025-03-02"));
    let clear = PeriodPatch {
        sent_date: Some(None),
        ..PeriodPatch::default()
    };
    let cleared = period::update(&mut c, &s.scope, id, &clear).await;
    let cleared = cleared.unwrap().unwrap();
    assert_eq!(cleared.sent_date, None);
    assert_eq!(cleared.billing_date.as_deref(), Some("2025-03-01"));

    let foreign = period::update(&mut c, &other.scope, id, &dates).await;
    assert!(matches!(foreign, Err(DbError::NotFound)));
    let foreign = period::delete(&mut c, &other.scope, id).await;
    assert!(matches!(foreign, Err(DbError::NotFound)));
    period::delete(&mut c, &s.scope, id).await.unwrap();
    let gone = period::get(&mut c, &s.scope, id).await;
    assert!(matches!(gone, Err(DbError::NotFound)));
}
dual_test!(period_rules, periods);

/// A patch that sets only the total.
fn total(cents: Option<i64>) -> CostPatch {
    CostPatch {
        total_amount_cents: Some(cents),
        ..CostPatch::default()
    }
}

/// Costs: create, `null` total, partial update, review rule, version, unit amounts.
async fn costs(db: &Db) {
    let s = setup(db, "A").await;
    let other = setup(db, "B").await;
    let mut c = db.pool().acquire().await.unwrap();
    let (pid, cat) = (s.period.id.as_str(), s.category_id.as_str());

    let created = cost::save(&mut c, &s.scope, pid, cat, &total(Some(12_345))).await;
    let Saved::Created(first) = created.unwrap().unwrap() else {
        panic!("neu angelegt erwartet")
    };
    assert!(first.reviewed && first.enabled);
    assert_eq!(
        (first.version, first.category_key),
        (1, DistributionKey::Mea)
    );

    let confirm = CostPatch {
        reviewed: Some(false),
        ..CostPatch::default()
    };
    cost::save(&mut c, &s.scope, pid, cat, &confirm)
        .await
        .unwrap()
        .unwrap();
    let confirm = CostPatch {
        reviewed: Some(true),
        version: Some(2),
        ..CostPatch::default()
    };
    let saved = cost::save(&mut c, &s.scope, pid, cat, &confirm).await;
    let saved = saved.unwrap().unwrap().value().clone();
    assert!(saved.reviewed);
    assert_eq!(saved.version, 3);

    // Same amount again: nothing changes, not even the version.
    let same = cost::save(&mut c, &s.scope, pid, cat, &total(Some(12_345))).await;
    let same = same.unwrap().unwrap().value().clone();
    assert_eq!((same.reviewed, same.version), (true, 3));
    // Another amount: unreviewed; the other fields stay.
    let toggle = CostPatch {
        enabled: Some(false),
        override_key: Some(Some(DistributionKey::PerUnit)),
        ..CostPatch::default()
    };
    cost::save(&mut c, &s.scope, pid, cat, &toggle)
        .await
        .unwrap()
        .unwrap();
    let cleared = cost::save(&mut c, &s.scope, pid, cat, &total(None)).await;
    let cleared = cleared.unwrap().unwrap().value().clone();
    assert_eq!(cleared.total_amount_cents, None);
    assert!(!cleared.reviewed && !cleared.enabled);
    assert_eq!(cleared.key(), DistributionKey::PerUnit);

    let stale = CostPatch {
        version: Some(1),
        ..total(Some(1))
    };
    let conflict = cost::save(&mut c, &s.scope, pid, cat, &stale).await;
    match conflict.unwrap().unwrap_err() {
        Rejection::CostVersion(current) => assert_eq!(current.total_amount_cents, None),
        other => panic!("Versionskonflikt erwartet: {other:?}"),
    }

    let foreign = cost::save(&mut c, &other.scope, pid, cat, &total(Some(1))).await;
    assert!(matches!(foreign, Err(DbError::NotFound)));
    let foreign_cat = cost::save(&mut c, &s.scope, pid, &other.category_id, &total(None)).await;
    assert!(matches!(foreign_cat, Err(DbError::NotFound)));
    assert!(matches!(
        cost::list(&mut c, &other.scope, pid).await,
        Err(DbError::NotFound)
    ));
}
dual_test!(cost_rules, costs);

/// Prepayments: amount required when new, review rule, version, foreign unit.
async fn prepayments(db: &Db) {
    let s = setup(db, "A").await;
    let other = setup(db, "B").await;
    let mut c = db.pool().acquire().await.unwrap();
    let (pid, uid) = (s.period.id.as_str(), s.units[0].as_str());
    let amount = |cents| PrepaymentPatch {
        monthly_amount_cents: Some(cents),
        ..PrepaymentPatch::default()
    };

    let refused = prepayment::save(&mut c, &s.scope, pid, uid, &PrepaymentPatch::default()).await;
    assert_eq!(refused.unwrap().unwrap_err(), Rejection::AmountRequired);
    let created = prepayment::save(&mut c, &s.scope, pid, uid, &amount(15_000)).await;
    assert!(matches!(created.unwrap().unwrap(), Saved::Created(p) if p.reviewed));
    let same = prepayment::save(&mut c, &s.scope, pid, uid, &amount(15_000)).await;
    assert_eq!(same.unwrap().unwrap().value().version, 1);
    let changed = prepayment::save(&mut c, &s.scope, pid, uid, &amount(16_000)).await;
    let changed = changed.unwrap().unwrap().value().clone();
    assert_eq!((changed.version, changed.reviewed), (2, false));
    let stale = PrepaymentPatch {
        version: Some(1),
        reviewed: Some(true),
        ..PrepaymentPatch::default()
    };
    let conflict = prepayment::save(&mut c, &s.scope, pid, uid, &stale).await;
    assert!(matches!(
        conflict.unwrap(),
        Err(Rejection::PrepaymentVersion(p)) if p.monthly_amount_cents == 16_000
    ));
    let list = prepayment::list(&mut c, &s.scope, pid).await.unwrap();
    assert_eq!(list.len(), 1);

    let foreign_unit = prepayment::save(&mut c, &s.scope, pid, &other.units[0], &amount(1)).await;
    assert!(matches!(foreign_unit, Err(DbError::NotFound)));
    let foreign = prepayment::save(&mut c, &other.scope, pid, uid, &amount(1)).await;
    assert!(matches!(foreign, Err(DbError::NotFound)));
}
dual_test!(prepayment_rules, prepayments);
