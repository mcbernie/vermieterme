//! Cost positions, unit amounts and prepayments over the real router: `PATCH` with only
//! the sent fields, `null` clears, the review rule, 409 `version_conflict` with
//! `current`, parallel saves, foreign ids 404 and the audit entries of an admin.

// Test bodies and helpers are plain async fns run by `dual_test!`.
#![allow(clippy::unwrap_used)]

mod billing_http;
mod masterdata_support;
mod support;

use serde_json::{Value, json};
use vermieterme_db::{Db, dual_test};

use billing_http::{billing, category};
use masterdata_support::world;
use support::audit_count;

/// Costs: create (201), change (200), `null`, review rule, version conflict, validation,
/// unit amounts, foreign ids.
async fn costs(db: &Db) {
    let w = world(db).await;
    let b = billing(&w, &w.a).await;
    let cat = category(&w, &w.a, "Grundsteuer", "mea").await;
    let path = b.path(&format!("/costs/{cat}"));

    let first = w
        .expect(
            201,
            "PATCH",
            &path,
            &w.a,
            json!({"totalAmountCents": 12_345}),
        )
        .await;
    assert_eq!(first.body["categoryName"], "Grundsteuer");
    assert_eq!(first.body["distributionKey"], "mea");
    assert_eq!(first.body["distributionKeyOverride"], Value::Null);
    assert_eq!(
        (
            first.body["reviewed"].clone(),
            first.body["version"].clone()
        ),
        (json!(true), json!(1))
    );
    let same = w
        .expect(
            200,
            "PATCH",
            &path,
            &w.a,
            json!({"totalAmountCents": 12_345}),
        )
        .await;
    assert_eq!(same.body["version"], 1);
    let key = json!({"distributionKeyOverride": "per_unit", "version": 1});
    let changed = w.expect(200, "PATCH", &path, &w.a, key).await;
    assert_eq!(changed.body["distributionKey"], "per_unit");
    assert_eq!(changed.body["reviewed"], true);
    assert_eq!(changed.body["totalAmountCents"], 12_345);
    let cleared = w
        .expect(200, "PATCH", &path, &w.a, json!({"totalAmountCents": null}))
        .await;
    assert_eq!(cleared.body["totalAmountCents"], Value::Null);
    assert_eq!(cleared.body["reviewed"], false);
    assert_eq!(cleared.body["version"], 3);

    let stale = json!({"reviewed": true, "version": 2});
    let conflict = w.expect(409, "PATCH", &path, &w.a, stale).await;
    assert_eq!(conflict.body["code"], "version_conflict");
    assert_eq!(conflict.body["current"]["version"], 3);
    assert_eq!(conflict.body["current"]["totalAmountCents"], Value::Null);

    let bad = json!({"totalAmountCents": 2_000_000_000_000_i64, "distributionKeyOverride": "x"});
    let resp = w.expect(400, "PATCH", &path, &w.a, bad).await;
    assert!(resp.body["fields"]["totalAmountCents"].is_string());
    assert!(resp.body["fields"]["distributionKeyOverride"].is_string());

    let unit_path = b.path(&format!("/costs/{cat}/units/{}", b.units[1]));
    let set = w
        .expect(200, "PATCH", &unit_path, &w.a, json!({"amountCents": 999}))
        .await;
    assert_eq!(
        set.body["unitAmounts"],
        json!([{"unitId": b.units[1], "amountCents": 999}])
    );
    w.expect(400, "PATCH", &unit_path, &w.a, json!({})).await;
    let clear = w
        .expect(200, "PATCH", &unit_path, &w.a, json!({"amountCents": null}))
        .await;
    assert_eq!(clear.body["unitAmounts"], json!([]));
    let elsewhere = w.property(&w.a).await;
    let stranger = w.unit(&w.a, &elsewhere, 1).await;
    let stranger_path = b.path(&format!("/costs/{cat}/units/{stranger}"));
    w.expect(
        404,
        "PATCH",
        &stranger_path,
        &w.a,
        json!({"amountCents": 1}),
    )
    .await;

    let list = w.app.get(&b.path("/costs"), Some(&w.a)).await;
    assert_eq!(list.body.as_array().unwrap().len(), 1);
    assert_eq!(w.app.get(&b.path("/costs"), Some(&w.b)).await.status, 404);
    w.expect(404, "PATCH", &path, &w.b, json!({"enabled": false}))
        .await;
    w.expect(404, "PATCH", &unit_path, &w.b, json!({"amountCents": 1}))
        .await;
    let other_cat = category(&w, &w.b, "Fremd", "mea").await;
    let foreign_cat = b.path(&format!("/costs/{other_cat}"));
    w.expect(404, "PATCH", &foreign_cat, &w.a, json!({"enabled": false}))
        .await;
    // A category in use cannot be deleted (409, not a cascade).
    let in_use = format!("/api/cost-categories/{cat}");
    w.expect(409, "DELETE", &in_use, &w.a, Value::Null).await;
}
dual_test!(cost_flow, costs);

/// Prepayments: amount required when new, create (201), review rule, conflict, foreign
/// ids.
async fn prepayments(db: &Db) {
    let w = world(db).await;
    let b = billing(&w, &w.a).await;
    let path = b.path(&format!("/prepayments/{}", b.units[0]));

    let resp = w
        .expect(400, "PATCH", &path, &w.a, json!({"reviewed": true}))
        .await;
    assert!(resp.body["fields"]["monthlyAmountCents"].is_string());
    w.expect(400, "PATCH", &path, &w.a, json!({"monthlyAmountCents": -1}))
        .await;
    let created = w
        .expect(
            201,
            "PATCH",
            &path,
            &w.a,
            json!({"monthlyAmountCents": 15_000}),
        )
        .await;
    assert_eq!(created.body["unitId"], b.units[0].as_str());
    assert_eq!(created.body["reviewed"], true);
    let changed = w
        .expect(
            200,
            "PATCH",
            &path,
            &w.a,
            json!({"monthlyAmountCents": 16_000, "version": 1}),
        )
        .await;
    assert_eq!(
        (
            changed.body["reviewed"].clone(),
            changed.body["version"].clone()
        ),
        (json!(false), json!(2))
    );
    let conflict = w
        .expect(
            409,
            "PATCH",
            &path,
            &w.a,
            json!({"reviewed": true, "version": 1}),
        )
        .await;
    assert_eq!(conflict.body["current"]["monthlyAmountCents"], 16_000);
    let list = w.app.get(&b.path("/prepayments"), Some(&w.a)).await;
    assert_eq!(list.body.as_array().unwrap().len(), 1);

    assert_eq!(
        w.app.get(&b.path("/prepayments"), Some(&w.b)).await.status,
        404
    );
    w.expect(404, "PATCH", &path, &w.b, json!({"monthlyAmountCents": 1}))
        .await;
    let other = w.property(&w.b).await;
    let foreign_unit = w.unit(&w.b, &other, 1).await;
    let foreign_path = b.path(&format!("/prepayments/{foreign_unit}"));
    w.expect(
        404,
        "PATCH",
        &foreign_path,
        &w.a,
        json!({"monthlyAmountCents": 1}),
    )
    .await;
}
dual_test!(prepayment_flow, prepayments);

/// Two saves of different fields of one position at the same time keep both (#25).
async fn parallel_saves(db: &Db) {
    let w = world(db).await;
    let b = billing(&w, &w.a).await;
    for round in 0..4 {
        let cat = category(&w, &w.a, &format!("K{round}"), "mea").await;
        let path = b.path(&format!("/costs/{cat}"));
        let (x, y) = tokio::join!(
            w.app
                .call("PATCH", &path, Some(&w.a), json!({"enabled": false})),
            w.app
                .call("PATCH", &path, Some(&w.a), json!({"totalAmountCents": 700})),
        );
        assert!(x.status < 300 && y.status < 300, "{x:?} / {y:?}");
        let list = w.app.get(&b.path("/costs"), Some(&w.a)).await.body;
        let saved = list
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["costCategoryId"] == cat);
        let saved = saved.unwrap();
        assert_eq!(
            (saved["enabled"].clone(), saved["totalAmountCents"].clone()),
            (json!(false), json!(700))
        );
    }
}
dual_test!(parallel_flow, parallel_saves);

/// An admin working inside the customer leaves one audit entry per change.
async fn audit(db: &Db) {
    let w = world(db).await;
    w.admin_enters_a().await;
    let b = billing(&w, &w.admin).await;
    let cat = category(&w, &w.admin, "Grundsteuer", "mea").await;
    let cost = b.path(&format!("/costs/{cat}"));
    w.expect(
        201,
        "PATCH",
        &cost,
        &w.admin,
        json!({"totalAmountCents": 1}),
    )
    .await;
    let unit = b.path(&format!("/costs/{cat}/units/{}", b.units[0]));
    w.expect(200, "PATCH", &unit, &w.admin, json!({"amountCents": 1}))
        .await;
    let prepay = b.path(&format!("/prepayments/{}", b.units[0]));
    w.expect(
        201,
        "PATCH",
        &prepay,
        &w.admin,
        json!({"monthlyAmountCents": 1}),
    )
    .await;
    w.expect(
        200,
        "PUT",
        &b.path(""),
        &w.admin,
        json!({"sentDate": "2025-01-01"}),
    )
    .await;
    w.expect(204, "DELETE", &b.path(""), &w.admin, Value::Null)
        .await;
    for action in [
        "billing_period.create",
        "billing_period.update",
        "billing_period.delete",
        "cost.save",
        "cost.save_unit_amount",
        "prepayment.save",
    ] {
        assert_eq!(audit_count(db, action).await, 1, "{action}");
    }
    // The customer's own changes are not logged.
    let own = billing(&w, &w.a).await;
    w.expect(204, "DELETE", &own.path(""), &w.a, Value::Null)
        .await;
    assert_eq!(audit_count(db, "billing_period.delete").await, 1);
}
dual_test!(audit_flow, audit);
