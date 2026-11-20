//! Audit coverage of master data: an admin inside a customer leaves exactly one entry
//! per change of every kind; a refused change leaves none.

// Test bodies and helpers are plain async fns run by `dual_test!`.
#![allow(clippy::unwrap_used)]

mod masterdata_support;
mod support;

use serde_json::{Value, json};
use support::audit_count;
use vermieterme_db::{Db, dual_test};

use masterdata_support::{World, id_of, world};

/// A tenant body for `unit_id`.
fn tenant_body(unit_id: &str) -> Value {
    json!({"unitId": unit_id, "salutation": "Herr", "firstName": "A", "lastName": "B",
        "moveInDate": "2024-01-01"})
}

/// A rent change body for `unit_id`.
fn change_body(unit_id: &str) -> Value {
    json!({"unitId": unit_id, "type": "rent", "amountCents": 1000, "effectiveDate": "2024-01-01"})
}

/// Inserts a cost of the category in a billing period of the property.
async fn cost_of(db: &Db, property_id: &str, category_id: &str) {
    let mut conn = db.pool().acquire().await.unwrap();
    sqlx::query(
        "INSERT INTO billing_period (id, account_id, property_id, start_date, end_date, \
         created_at, updated_at) VALUES ('bp9', (SELECT account_id FROM property WHERE id = ?), \
         ?, '2024-01-01', '2024-12-31', 1, 1)",
    )
    .bind(property_id)
    .bind(property_id)
    .execute(&mut *conn)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO cost (id, account_id, billing_period_id, cost_category_id, created_at, \
         updated_at) VALUES ('c9', (SELECT account_id FROM property WHERE id = ?), 'bp9', ?, 1, 1)",
    )
    .bind(property_id)
    .bind(category_id)
    .execute(&mut *conn)
    .await
    .unwrap();
}

/// One DELETE with an expected status.
async fn delete(w: &World, path: &str, status: u16) {
    w.expect(status, "DELETE", path, &w.admin, Value::Null)
        .await;
}

/// An admin inside a customer leaves exactly one audit entry per change of every master
/// data kind; a refused deletion of a category in use leaves none.
async fn audit_of_all_actions(db: &Db) {
    let w = world(db).await;
    w.admin_enters_a().await;
    let pid = w.property(&w.admin).await;
    let uid = w.unit(&w.admin, &pid, 10).await;
    let tid = w.tenant(&w.admin, &uid, json!({})).await;
    let admin = &w.admin;

    let property = json!({"street": "X", "zip": "1", "city": "Y", "totalShares": 100});
    w.expect(
        200,
        "PUT",
        &format!("/api/properties/{pid}"),
        admin,
        property,
    )
    .await;
    let unit = json!({"propertyId": pid, "name": "W", "floor": "", "shares": 10});
    w.expect(200, "PUT", &format!("/api/units/{uid}"), admin, unit)
        .await;
    w.expect(
        200,
        "PUT",
        &format!("/api/tenants/{tid}"),
        admin,
        tenant_body(&uid),
    )
    .await;
    let change = w
        .expect(201, "POST", "/api/rent-changes", admin, change_body(&uid))
        .await;
    let change_path = format!("/api/rent-changes/{}", id_of(&change));
    w.expect(200, "PUT", &change_path, admin, change_body(&uid))
        .await;
    let category = json!({"name": "Wasser", "distributionKey": "mea", "sortOrder": 1});
    let cat = w
        .expect(201, "POST", "/api/cost-categories", admin, category.clone())
        .await;
    let cat_path = format!("/api/cost-categories/{}", id_of(&cat));
    w.expect(200, "PUT", &cat_path, admin, category).await;
    let landlord = json!({"name": "Max", "street": "Weg 2", "zip": "1", "city": "S"});
    w.expect(200, "PUT", "/api/settings", admin, landlord).await;
    let template = w.app.get("/api/pdf-template", Some(admin)).await;
    w.expect(
        200,
        "PUT",
        "/api/pdf-template",
        admin,
        json!({"config": template.body["config"]}),
    )
    .await;

    cost_of(db, &pid, &id_of(&cat)).await;
    delete(&w, &cat_path, 409).await;
    assert_eq!(audit_count(db, "cost_category.delete").await, 0);
    let mut conn = db.pool().acquire().await.unwrap();
    sqlx::query("DELETE FROM cost WHERE id = 'c9'")
        .execute(&mut *conn)
        .await
        .unwrap();
    drop(conn);
    delete(&w, &cat_path, 204).await;
    delete(&w, &change_path, 204).await;
    delete(&w, &format!("/api/tenants/{tid}"), 204).await;
    delete(&w, &format!("/api/units/{uid}"), 204).await;
    delete(&w, &format!("/api/properties/{pid}"), 204).await;

    for action in [
        "property.create",
        "property.update",
        "property.delete",
        "unit.create",
        "unit.update",
        "unit.delete",
        "tenant.create",
        "tenant.update",
        "tenant.delete",
        "rent_change.create",
        "rent_change.update",
        "rent_change.delete",
        "cost_category.create",
        "cost_category.update",
        "cost_category.delete",
        "landlord.save",
        "pdf_template.save",
    ] {
        assert_eq!(audit_count(db, action).await, 1, "{action}");
    }
}
dual_test!(audit_covers_all_actions, audit_of_all_actions);
