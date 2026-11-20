//! Properties, units, tenants and rent changes over the real router: statuses, fields,
//! validation (400), foreign ids (404), the share rules and the audit entries of an admin
//! working inside a customer.

// Test bodies and helpers are plain async fns run by `dual_test!`.
#![allow(clippy::unwrap_used)]

mod masterdata_support;
mod support;

use serde_json::json;
use vermieterme_db::{Db, dual_test};

use masterdata_support::{id_of, world};

/// Property: create, list with `unitCount`, detail with units and tenants, replace,
/// delete; validation; the other customer sees nothing and gets 404 everywhere.
async fn properties(db: &Db) {
    let w = world(db).await;
    let pid = w.property(&w.a).await;
    let uid = w.unit(&w.a, &pid, 40).await;
    w.tenant(&w.a, &uid, json!({})).await;
    let path = format!("/api/properties/{pid}");

    let list = w.app.get("/api/properties", Some(&w.a)).await;
    assert_eq!(list.body[0]["unitCount"], 1);
    assert_eq!(list.body[0]["totalShares"], 100);
    let detail = w.app.get(&path, Some(&w.a)).await;
    assert_eq!(detail.body["units"][0]["shares"], 40);
    assert_eq!(detail.body["units"][0]["kind"], "apartment");
    assert_eq!(detail.body["units"][0]["tenants"][0]["firstName"], "Erika");

    assert_eq!(
        w.app.get("/api/properties", Some(&w.b)).await.body,
        json!([])
    );
    assert_eq!(w.app.get(&path, Some(&w.b)).await.status, 404);
    let body = json!({"street": "X", "zip": "1", "city": "Y", "totalShares": 100});
    w.expect(404, "PUT", &path, &w.b, body.clone()).await;
    w.expect(404, "DELETE", &path, &w.b, serde_json::Value::Null)
        .await;
    assert_eq!(
        w.app
            .get("/api/properties", Some(&w.a))
            .await
            .body
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(w.app.get("/api/properties", None).await.status, 401);

    let bad = json!({"street": "  ", "zip": "x".repeat(17), "city": "Y", "totalShares": 0});
    let resp = w.expect(400, "POST", "/api/properties", &w.a, bad).await;
    assert_eq!(resp.body["code"], "validation");
    for field in ["street", "zip", "totalShares"] {
        assert!(resp.body["fields"][field].is_string(), "{field}");
    }
    let lower = json!({"street": "X", "zip": "1", "city": "Y", "totalShares": 39});
    let resp = w.expect(400, "PUT", &path, &w.a, lower).await;
    assert!(resp.body["fields"]["totalShares"].is_string());

    let body = json!({"street": "  Neuweg 2 ", "zip": "54321", "city": "Y", "totalShares": 40});
    let resp = w.expect(200, "PUT", &path, &w.a, body).await;
    assert_eq!(resp.body["street"], "Neuweg 2");
    w.expect(204, "DELETE", &path, &w.a, serde_json::Value::Null)
        .await;
    assert_eq!(w.app.get(&path, Some(&w.a)).await.status, 404);
    assert_eq!(w.app.get("/api/units", Some(&w.a)).await.body, json!([]));
}
dual_test!(properties_crud_and_isolation, properties);

/// Units: kind, share rules against the property, foreign property and filter -> 404.
async fn units(db: &Db) {
    let w = world(db).await;
    let pid = w.property(&w.a).await;
    let uid = w.unit(&w.a, &pid, 60).await;
    let path = format!("/api/units/{uid}");
    let body = |shares: i64| json!({"propertyId": pid, "name": "W2", "floor": "1. OG", "shares": shares, "kind": "garage"});

    let over = w.expect(400, "POST", "/api/units", &w.a, body(101)).await;
    assert!(over.body["fields"]["shares"].is_string());
    let neg = w.expect(400, "POST", "/api/units", &w.a, body(-1)).await;
    assert!(neg.body["fields"]["shares"].is_string());
    let bad_kind =
        json!({"propertyId": pid, "name": "W", "floor": "", "shares": 1, "kind": "villa"});
    let resp = w.expect(400, "POST", "/api/units", &w.a, bad_kind).await;
    assert!(resp.body["fields"]["kind"].is_string());
    let missing_name = json!({"propertyId": pid, "name": " ", "floor": "", "shares": 1});
    w.expect(400, "POST", "/api/units", &w.a, missing_name)
        .await;
    w.expect(400, "PUT", &path, &w.a, body(101)).await;

    let resp = w.expect(200, "PUT", &path, &w.a, body(100)).await;
    assert_eq!(
        (resp.body["kind"].as_str(), resp.body["shares"].as_i64()),
        (Some("garage"), Some(100))
    );
    let list = w
        .app
        .get(&format!("/api/units?propertyId={pid}"), Some(&w.a))
        .await;
    assert_eq!(list.body.as_array().unwrap().len(), 1);

    // The other customer: foreign property, unit and filter behave like missing ones.
    w.expect(404, "POST", "/api/units", &w.b, body(1)).await;
    w.expect(404, "PUT", &path, &w.b, body(1)).await;
    w.expect(404, "DELETE", &path, &w.b, serde_json::Value::Null)
        .await;
    assert_eq!(w.app.get("/api/units", Some(&w.b)).await.body, json!([]));
    let foreign_filter = w
        .app
        .get(&format!("/api/units?propertyId={pid}"), Some(&w.b))
        .await;
    assert_eq!(foreign_filter.status, 404);
    let foreign_move =
        json!({"propertyId": w.property(&w.b).await, "name": "W", "floor": "", "shares": 1});
    w.expect(404, "PUT", &path, &w.a, foreign_move).await;

    w.expect(204, "DELETE", &path, &w.a, serde_json::Value::Null)
        .await;
    w.expect(404, "DELETE", &path, &w.a, serde_json::Value::Null)
        .await;
}
dual_test!(units_rules_and_isolation, units);

/// Tenants: IBAN, e-mail, dates and lease rules are validated, foreign ids are 404.
async fn tenants(db: &Db) {
    let w = world(db).await;
    let pid = w.property(&w.a).await;
    let uid = w.unit(&w.a, &pid, 10).await;
    let extra = json!({"iban": "de89370400440532013000", "email": " Erika@Example.org ", "phone": "  ",
        "leaseType": "index", "indexBaseYear": 2020, "indexReferenceValueTenths": 1000});
    let tid = w.tenant(&w.a, &uid, extra).await;
    let path = format!("/api/tenants/{tid}");

    let list = w
        .app
        .get(&format!("/api/tenants?unitId={uid}"), Some(&w.a))
        .await;
    let t = &list.body[0];
    assert_eq!(t["iban"], "DE89 3704 0044 0532 0130 00");
    assert_eq!(t["email"], "erika@example.org");
    assert!(t["phone"].is_null());
    assert_eq!(t["indexReferenceValueTenths"], 1000);
    assert_eq!(t["indexMinMonths"], 12);

    let base = |patch: serde_json::Value| {
        let mut body = json!({"unitId": uid, "salutation": "Herr", "firstName": "A", "lastName": "B", "moveInDate": "2024-01-01"});
        body.as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        body
    };
    for (patch, field) in [
        (json!({"iban": "DE00 1234"}), "iban"),
        (json!({"email": "kein-mail"}), "email"),
        (json!({"moveInDate": "2024-02-30"}), "moveInDate"),
        (json!({"moveOutDate": "2023-12-31"}), "moveOutDate"),
        (json!({"leaseType": "pacht"}), "leaseType"),
        (json!({"indexBaseYear": 1999}), "indexBaseYear"),
        (json!({"indexMinMonths": -1}), "indexMinMonths"),
        (json!({"firstName": ""}), "firstName"),
        (json!({"lastName": "x".repeat(256)}), "lastName"),
    ] {
        let resp = w
            .expect(400, "POST", "/api/tenants", &w.a, base(patch))
            .await;
        assert!(resp.body["fields"][field].is_string(), "{field}");
    }
    let resp = w
        .expect(
            200,
            "PUT",
            &path,
            &w.a,
            base(json!({"moveOutDate": "2025-06-30"})),
        )
        .await;
    assert_eq!(resp.body["moveOutDate"], "2025-06-30");
    assert!(resp.body["iban"].is_null(), "PUT ersetzt alle Felder");

    w.expect(404, "POST", "/api/tenants", &w.b, base(json!({})))
        .await;
    w.expect(404, "PUT", &path, &w.b, base(json!({}))).await;
    w.expect(404, "DELETE", &path, &w.b, serde_json::Value::Null)
        .await;
    assert_eq!(w.app.get("/api/tenants", Some(&w.b)).await.body, json!([]));
    assert_eq!(
        w.app
            .get(&format!("/api/tenants?unitId={uid}"), Some(&w.b))
            .await
            .status,
        404
    );
    w.expect(204, "DELETE", &path, &w.a, serde_json::Value::Null)
        .await;
}
dual_test!(tenants_validation_and_isolation, tenants);

/// Rent changes: cents, type, date, newest first, foreign ids -> 404.
async fn rent_changes(db: &Db) {
    let w = world(db).await;
    let pid = w.property(&w.a).await;
    let uid = w.unit(&w.a, &pid, 10).await;
    let body = |amount: i64, date: &str| json!({"unitId": uid, "type": "rent", "amountCents": amount, "effectiveDate": date, "reason": " Erhöhung "});
    let first = w
        .expect(
            201,
            "POST",
            "/api/rent-changes",
            &w.a,
            body(65_000, "2024-01-01"),
        )
        .await;
    w.expect(
        201,
        "POST",
        "/api/rent-changes",
        &w.a,
        body(70_000, "2025-01-01"),
    )
    .await;
    assert_eq!(first.body["reason"], "Erhöhung");
    let path = format!("/api/rent-changes/{}", id_of(&first));

    let list = w
        .app
        .get(&format!("/api/rent-changes?unitId={uid}"), Some(&w.a))
        .await;
    assert_eq!(list.body[0]["amountCents"], 70_000);
    assert_eq!(list.body[0]["type"], "rent");

    let bad =
        json!({"unitId": uid, "type": "kaution", "amountCents": -5, "effectiveDate": "morgen"});
    let resp = w.expect(400, "POST", "/api/rent-changes", &w.a, bad).await;
    for field in ["type", "amountCents", "effectiveDate"] {
        assert!(resp.body["fields"][field].is_string(), "{field}");
    }
    let updated = w
        .expect(200, "PUT", &path, &w.a, body(66_000, "2024-02-01"))
        .await;
    assert_eq!(updated.body["amountCents"], 66_000);

    w.expect(
        404,
        "POST",
        "/api/rent-changes",
        &w.b,
        body(1, "2024-01-01"),
    )
    .await;
    w.expect(404, "PUT", &path, &w.b, body(1, "2024-01-01"))
        .await;
    w.expect(404, "DELETE", &path, &w.b, serde_json::Value::Null)
        .await;
    assert_eq!(
        w.app.get("/api/rent-changes", Some(&w.b)).await.body,
        json!([])
    );
    let foreign = w
        .app
        .get(&format!("/api/rent-changes?unitId={uid}"), Some(&w.b))
        .await;
    assert_eq!(foreign.status, 404);
    w.expect(204, "DELETE", &path, &w.a, serde_json::Value::Null)
        .await;
}
dual_test!(rent_changes_validation_and_isolation, rent_changes);
