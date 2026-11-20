//! Follow-ups of the master data review: embedded units (decision 38), tenant move and
//! unit move rules (36, 37), filters, target ids, 404 messages, complete audit coverage.

// Test bodies and helpers are plain async fns run by `dual_test!`.
#![allow(clippy::unwrap_used)]

mod masterdata_support;
mod support;

use serde_json::{Value, json};
use sqlx::Row;
use vermieterme_db::{Db, dual_test};

use masterdata_support::{id_of, world};

/// A tenant body for `unit_id`.
fn tenant_body(unit_id: &str) -> Value {
    json!({"unitId": unit_id, "salutation": "Herr", "firstName": "A", "lastName": "B",
        "moveInDate": "2024-01-01"})
}

/// A rent change body for `unit_id`.
fn change_body(unit_id: &str) -> Value {
    json!({"unitId": unit_id, "type": "rent", "amountCents": 1000, "effectiveDate": "2024-01-01"})
}

/// Lists, creations and replacements of tenants and rent changes carry
/// `unit{id,name,floor,shares,property{id,street,zip,city}}`; the other customer sees none.
async fn embedded_unit(db: &Db) {
    let w = world(db).await;
    let pid = w.property(&w.a).await;
    let uid = w.unit(&w.a, &pid, 40).await;
    let expected = json!({"id": uid, "name": "Wohnung 1", "floor": "EG", "shares": 40,
        "property": {"id": pid, "street": "Musterweg 1", "zip": "12345", "city": "Beispielstadt"}});

    let created = w
        .expect(201, "POST", "/api/tenants", &w.a, tenant_body(&uid))
        .await;
    assert_eq!(created.body["unit"], expected);
    let tid = id_of(&created);
    let list = w.app.get("/api/tenants", Some(&w.a)).await;
    assert_eq!(list.body[0]["unit"], expected);
    let put = w
        .expect(
            200,
            "PUT",
            &format!("/api/tenants/{tid}"),
            &w.a,
            tenant_body(&uid),
        )
        .await;
    assert_eq!(put.body["unit"], expected);

    let created = w
        .expect(201, "POST", "/api/rent-changes", &w.a, change_body(&uid))
        .await;
    assert_eq!(created.body["unit"], expected);
    let list = w.app.get("/api/rent-changes", Some(&w.a)).await;
    assert_eq!(list.body[0]["unit"], expected);
    let path = format!("/api/rent-changes/{}", id_of(&created));
    let put = w.expect(200, "PUT", &path, &w.a, change_body(&uid)).await;
    assert_eq!(put.body["unit"], expected);

    assert_eq!(w.app.get("/api/tenants", Some(&w.b)).await.body, json!([]));
    assert_eq!(
        w.app.get("/api/rent-changes", Some(&w.b)).await.body,
        json!([])
    );
}
dual_test!(unit_is_embedded, embedded_unit);

/// `app_tokens_revoked_at` of a tenant.
async fn revoked_at(db: &Db, id: &str) -> Option<i64> {
    let mut conn = db.pool().acquire().await.unwrap();
    sqlx::query("SELECT app_tokens_revoked_at AS r FROM tenant WHERE id = ?")
        .bind(id)
        .fetch_one(&mut *conn)
        .await
        .unwrap()
        .get("r")
}

/// A tenant who moves to another unit loses the app tokens; an ordinary edit does not.
async fn tenant_move(db: &Db) {
    let w = world(db).await;
    let pid = w.property(&w.a).await;
    let first = w.unit(&w.a, &pid, 10).await;
    let second = w.unit(&w.a, &pid, 10).await;
    let tid = w.tenant(&w.a, &first, json!({})).await;
    let path = format!("/api/tenants/{tid}");
    w.expect(200, "PUT", &path, &w.a, tenant_body(&first)).await;
    assert_eq!(revoked_at(db, &tid).await, None);
    w.expect(200, "PUT", &path, &w.a, tenant_body(&second))
        .await;
    assert!(revoked_at(db, &tid).await.is_some());
}
dual_test!(tenant_move_revokes_tokens, tenant_move);

/// Creates a billing period of the property with a prepayment for the unit.
async fn prepayment(db: &Db, property_id: &str, unit_id: &str) {
    let mut conn = db.pool().acquire().await.unwrap();
    sqlx::query(
        "INSERT INTO billing_period (id, account_id, property_id, start_date, end_date, \
         created_at, updated_at) VALUES ('bp1', (SELECT account_id FROM property WHERE id = ?), \
         ?, '2024-01-01', '2024-12-31', 1, 1)",
    )
    .bind(property_id)
    .bind(property_id)
    .execute(&mut *conn)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO prepayment (id, account_id, billing_period_id, unit_id, \
         monthly_amount_cents, created_at, updated_at) VALUES ('pp1', \
         (SELECT account_id FROM property WHERE id = ?), 'bp1', ?, 5000, 1, 1)",
    )
    .bind(property_id)
    .bind(unit_id)
    .execute(&mut *conn)
    .await
    .unwrap();
}

/// A unit in billings stays in its property (409); a free one can move.
async fn unit_move(db: &Db) {
    let w = world(db).await;
    let p1 = w.property(&w.a).await;
    let p2 = w.property(&w.a).await;
    let used = w.unit(&w.a, &p1, 10).await;
    let free = w.unit(&w.a, &p1, 10).await;
    prepayment(db, &p1, &used).await;
    let body =
        |property: &str| json!({"propertyId": property, "name": "W", "floor": "", "shares": 10});
    let path = format!("/api/units/{used}");
    let resp = w.expect(409, "PUT", &path, &w.a, body(&p2)).await;
    assert_eq!(resp.body["code"], "conflict");
    assert!(
        resp.body["error"]
            .as_str()
            .unwrap()
            .contains("Abrechnungen")
    );
    w.expect(200, "PUT", &path, &w.a, body(&p1)).await;
    let moved = w
        .expect(200, "PUT", &format!("/api/units/{free}"), &w.a, body(&p2))
        .await;
    assert_eq!(moved.body["propertyId"], p2);
}
dual_test!(unit_move_in_billings, unit_move);

/// Blank filters mean "no filter", over-long ids are 404, the 404 names the missing
/// object, and a foreign target leaves the record unchanged.
async fn filters_and_targets(db: &Db) {
    let w = world(db).await;
    let pid = w.property(&w.a).await;
    let uid = w.unit(&w.a, &pid, 10).await;
    let tid = w.tenant(&w.a, &uid, json!({})).await;
    let change = w
        .expect(201, "POST", "/api/rent-changes", &w.a, change_body(&uid))
        .await;
    for path in [
        "/api/units?propertyId=",
        "/api/units?propertyId=%20",
        "/api/tenants?unitId=",
        "/api/rent-changes?unitId=",
    ] {
        let resp = w.app.get(path, Some(&w.a)).await;
        assert_eq!(resp.status, 200, "{path}");
        assert_eq!(resp.body.as_array().unwrap().len(), 1, "{path}");
    }

    let long = "x".repeat(37);
    let unit = json!({"propertyId": long, "name": "W", "floor": "", "shares": 1});
    let resp = w.expect(404, "POST", "/api/units", &w.a, unit).await;
    assert_eq!(resp.body["error"], "Objekt nicht gefunden");
    let resp = w
        .expect(404, "POST", "/api/tenants", &w.a, tenant_body(&long))
        .await;
    assert_eq!(resp.body["error"], "Einheit nicht gefunden");
    w.expect(404, "POST", "/api/rent-changes", &w.a, change_body(&long))
        .await;

    // Foreign target: the unit of the other customer.
    let other_p = w.property(&w.b).await;
    let other_u = w.unit(&w.b, &other_p, 10).await;
    let tenant_path = format!("/api/tenants/{tid}");
    let resp = w
        .expect(404, "PUT", &tenant_path, &w.a, tenant_body(&other_u))
        .await;
    assert_eq!(resp.body["error"], "Einheit nicht gefunden");
    let change_path = format!("/api/rent-changes/{}", id_of(&change));
    let resp = w
        .expect(404, "PUT", &change_path, &w.a, change_body(&other_u))
        .await;
    assert_eq!(resp.body["error"], "Einheit nicht gefunden");
    let unit_path = format!("/api/units/{uid}");
    let to_foreign = json!({"propertyId": other_p, "name": "W", "floor": "", "shares": 1});
    let resp = w.expect(404, "PUT", &unit_path, &w.a, to_foreign).await;
    assert_eq!(resp.body["error"], "Objekt nicht gefunden");
    // Unknown record: named after the record, not the target.
    let resp = w
        .expect(
            404,
            "PUT",
            "/api/tenants/gibtsnicht",
            &w.a,
            tenant_body(&uid),
        )
        .await;
    assert_eq!(resp.body["error"], "Mieter nicht gefunden");

    // Nothing changed.
    let tenants = w.app.get("/api/tenants", Some(&w.a)).await;
    assert_eq!(tenants.body[0]["unitId"], uid);
    let changes = w.app.get("/api/rent-changes", Some(&w.a)).await;
    assert_eq!(changes.body[0]["unitId"], uid);
    let units = w.app.get("/api/units", Some(&w.a)).await;
    assert_eq!(units.body[0]["propertyId"], pid);
}
dual_test!(filters_and_target_errors, filters_and_targets);
