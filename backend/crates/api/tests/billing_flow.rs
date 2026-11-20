//! Billing periods over the real router: create with validation, overlap and copy, the
//! list with counts, the detail with the totals of the domain (and 422 for broken
//! shares), date changes, delete; the other customer gets 404 everywhere.

// Test bodies and helpers are plain async fns run by `dual_test!`.
#![allow(clippy::unwrap_used)]

mod billing_http;
mod masterdata_support;
mod support;

use serde_json::{Value, json};
use vermieterme_db::{Db, dual_test};

use billing_http::{billing, category, totals_of};
use masterdata_support::{id_of, world};

/// Create, list, detail, change and delete; validation, overlap, foreign ids.
async fn periods(db: &Db) {
    let w = world(db).await;
    let b = billing(&w, &w.a).await;
    let base = "/api/billing-periods";

    let bad = json!({"propertyId": b.property, "startDate": "2025-12-31", "endDate": "2025-01-01"});
    let resp = w.expect(400, "POST", base, &w.a, bad).await;
    assert!(resp.body["fields"]["endDate"].is_string());
    let bad = json!({"propertyId": b.property, "startDate": "31.12.2025", "endDate": "x"});
    let resp = w.expect(400, "POST", base, &w.a, bad).await;
    assert!(resp.body["fields"]["startDate"].is_string());
    let overlap =
        json!({"propertyId": b.property, "startDate": "2024-06-01", "endDate": "2025-05-31"});
    let resp = w.expect(409, "POST", base, &w.a, overlap).await;
    assert_eq!(resp.body["code"], "overlap");
    assert!(
        resp.body["error"]
            .as_str()
            .unwrap()
            .contains("01.01.2024 – 31.12.2024")
    );
    let empty = w.property(&w.a).await;
    let none = json!({"propertyId": empty, "startDate": "2024-01-01", "endDate": "2024-12-31"});
    let resp = w.expect(400, "POST", base, &w.a, none).await;
    assert!(resp.body["fields"]["propertyId"].is_string());
    let foreign =
        json!({"propertyId": b.property, "startDate": "2030-01-01", "endDate": "2030-12-31"});
    w.expect(404, "POST", base, &w.b, foreign).await;

    let list = w.app.get(base, Some(&w.a)).await;
    assert_eq!(list.body[0]["id"], b.period.as_str());
    // `property{id, street, zip, city}` of the contract, nothing more.
    let embed = list.body[0]["property"].as_object().unwrap();
    let mut keys: Vec<&str> = embed.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["city", "id", "street", "zip"]);
    assert_eq!(embed["street"], "Musterweg 1");
    assert_eq!(list.body[0]["costCount"], 0);
    assert_eq!(list.body[0]["unreviewedCount"], 0);
    let filtered = w
        .app
        .get(&format!("{base}?propertyId={empty}"), Some(&w.a))
        .await;
    assert_eq!(filtered.body, json!([]));
    assert_eq!(w.app.get(base, Some(&w.b)).await.body, json!([]));

    let detail = w.app.get(&b.path(""), Some(&w.a)).await;
    assert_eq!(detail.status, 200);
    assert_eq!(detail.body["startDate"], "2024-01-01");
    assert_eq!(
        detail.body["property"]["units"].as_array().unwrap().len(),
        2
    );
    assert_eq!(totals_of(&detail.body, &b.units[0])["months"], 12);
    for method in ["GET", "PUT", "DELETE"] {
        let body = if method == "PUT" {
            json!({})
        } else {
            Value::Null
        };
        w.expect(404, method, &b.path(""), &w.b, body).await;
    }

    let dates = json!({"billingDate": "2025-03-01", "sentDate": "2025-03-02"});
    let changed = w.expect(200, "PUT", &b.path(""), &w.a, dates).await;
    assert_eq!(changed.body["sentDate"], "2025-03-02");
    let clear = json!({"sentDate": null, "endDate": "2024-06-30"});
    let changed = w.expect(200, "PUT", &b.path(""), &w.a, clear).await;
    assert_eq!(changed.body["sentDate"], Value::Null);
    assert_eq!(changed.body["billingDate"], "2025-03-01");
    // Change and create answer with the summary; the totals come with the detail.
    assert_eq!(changed.body["property"]["street"], "Musterweg 1");
    assert_eq!(changed.body["costCount"], 0);
    assert!(changed.body.get("totalsByUnit").is_none());
    let detail = w.app.get(&b.path(""), Some(&w.a)).await.body;
    assert_eq!(totals_of(&detail, &b.units[0])["months"], 6);
    let resp = w
        .expect(
            400,
            "PUT",
            &b.path(""),
            &w.a,
            json!({"endDate": "2023-01-01"}),
        )
        .await;
    assert!(resp.body["fields"]["endDate"].is_string());
    w.expect(400, "PUT", &b.path(""), &w.a, json!({"startDate": null}))
        .await;
    let next =
        json!({"propertyId": b.property, "startDate": "2024-07-01", "endDate": "2025-06-30"});
    let next = id_of(&w.expect(201, "POST", base, &w.a, next).await);
    let into_next = json!({"endDate": "2024-07-15"});
    let resp = w.expect(409, "PUT", &b.path(""), &w.a, into_next).await;
    assert_eq!(resp.body["code"], "overlap");

    w.expect(204, "DELETE", &format!("{base}/{next}"), &w.a, Value::Null)
        .await;
    w.expect(404, "GET", &format!("{base}/{next}"), &w.a, Value::Null)
        .await;
}
dual_test!(period_flow, periods);

/// Totals per unit through the domain: MEA from the total, `per_notice` from the unit
/// amount, deactivated costs nowhere, prepayment times calendar months; the copy takes
/// over the positions unreviewed.
async fn totals_and_copy(db: &Db) {
    let w = world(db).await;
    let b = billing(&w, &w.a).await;
    let mea = category(&w, &w.a, "Grundsteuer", "mea").await;
    let notice = category(&w, &w.a, "Wasser", "per_notice").await;
    let off = category(&w, &w.a, "Garten", "per_unit").await;
    let wr = &w;
    let save = |cat: &str, body: Value| {
        let path = b.path(&format!("/costs/{cat}"));
        async move { wr.expect(201, "PATCH", &path, &wr.a, body).await }
    };
    save(&mea, json!({"totalAmountCents": 100_001})).await;
    save(&off, json!({"totalAmountCents": 50_000, "enabled": false})).await;
    let unit_path = b.path(&format!("/costs/{notice}/units/{}", b.units[0]));
    w.expect(
        200,
        "PATCH",
        &unit_path,
        &w.a,
        json!({"amountCents": 7_000}),
    )
    .await;
    // Decision 39: by notice the unit amount counts even without a total.
    let detail = w.app.get(&b.path(""), Some(&w.a)).await.body;
    assert_eq!(
        totals_of(&detail, &b.units[0])["totalUnitCostsCents"],
        40_000 + 7_000
    );
    let notice_total = json!({"totalAmountCents": 7_000, "version": 1});
    let notice_path = b.path(&format!("/costs/{notice}"));
    w.expect(200, "PATCH", &notice_path, &w.a, notice_total)
        .await;
    let prepay = b.path(&format!("/prepayments/{}", b.units[0]));
    w.expect(
        201,
        "PATCH",
        &prepay,
        &w.a,
        json!({"monthlyAmountCents": 5_000}),
    )
    .await;

    let detail = w.app.get(&b.path(""), Some(&w.a)).await.body;
    let first = totals_of(&detail, &b.units[0]);
    // 40 % of 1000.01 € = 400.004 -> 400.00 €, plus 70.00 € by notice.
    assert_eq!(first["totalUnitCostsCents"], 40_000 + 7_000);
    assert_eq!(first["totalPrepaymentCents"], 60_000);
    assert_eq!(first["differenceCents"], 60_000 - 47_000);
    let second = totals_of(&detail, &b.units[1]);
    assert_eq!(second["totalUnitCostsCents"], 60_001);
    assert_eq!(second["totalPrepaymentCents"], 0);

    let copy = json!({"propertyId": b.property, "startDate": "2025-01-01",
        "endDate": "2025-12-31", "copyFromId": b.period});
    let copied = w
        .expect(201, "POST", "/api/billing-periods", &w.a, copy)
        .await
        .body;
    assert_eq!(copied["copiedFromId"], b.period.as_str());
    assert_eq!(
        (
            copied["costCount"].clone(),
            copied["unreviewedCount"].clone()
        ),
        (json!(3), json!(3))
    );
    let copy_path = format!("/api/billing-periods/{}", copied["id"].as_str().unwrap());
    let copied = w.app.get(&copy_path, Some(&w.a)).await.body;
    let costs = copied["costs"].as_array().unwrap();
    assert_eq!(costs.len(), 3);
    assert!(
        costs
            .iter()
            .all(|c| c["reviewed"] == false && c["version"] == 1)
    );
    let garden = costs.iter().find(|c| c["costCategoryId"] == off).unwrap();
    assert_eq!(garden["enabled"], false);
    let water = costs
        .iter()
        .find(|c| c["costCategoryId"] == notice)
        .unwrap();
    assert_eq!(water["unitAmounts"][0]["amountCents"], 7_000);
    assert_eq!(copied["prepayments"][0]["reviewed"], false);
    let list = w.app.get("/api/billing-periods", Some(&w.a)).await.body;
    assert_eq!(list[0]["costCount"], 3);
    // Two enabled costs and one prepayment are unreviewed in the copy.
    assert_eq!(list[0]["unreviewedCount"], 3);

    let foreign = json!({"propertyId": b.property, "startDate": "2026-01-01",
        "endDate": "2026-12-31", "copyFromId": "unbekannt"});
    w.expect(404, "POST", "/api/billing-periods", &w.a, foreign)
        .await;

    // Broken shares from old data: 422 naming the unit, only while MEA counts.
    let mut conn = db.pool().acquire().await.unwrap();
    sqlx::query("UPDATE unit SET shares = 500, name = 'Dachgeschoss' WHERE id = ?")
        .bind(&b.units[1])
        .execute(&mut *conn)
        .await
        .unwrap();
    let resp = w.app.get(&b.path(""), Some(&w.a)).await;
    assert_eq!(resp.status, 422);
    assert_eq!(resp.body["code"], "invalid_shares");
    assert!(
        resp.body["error"]
            .as_str()
            .unwrap()
            .contains("Dachgeschoss")
    );
    // A change still succeeds: it answers with the summary, not the detail.
    let sent = json!({"sentDate": "2025-02-01"});
    let changed = w.expect(200, "PUT", &b.path(""), &w.a, sent).await;
    assert_eq!(changed.body["sentDate"], "2025-02-01");
    let off_mea = json!({"enabled": false});
    w.expect(
        200,
        "PATCH",
        &b.path(&format!("/costs/{mea}")),
        &w.a,
        off_mea,
    )
    .await;
    assert_eq!(w.app.get(&b.path(""), Some(&w.a)).await.status, 200);
}
dual_test!(totals_flow, totals_and_copy);
