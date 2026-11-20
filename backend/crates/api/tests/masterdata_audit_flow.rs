//! Audit entries of master data changes: none for members, one per change for an admin
//! working inside a customer, none for refused changes.

// Test bodies and helpers are plain async fns run by `dual_test!`.
#![allow(clippy::unwrap_used)]

mod masterdata_support;
mod support;

use serde_json::json;
use support::audit_count;
use vermieterme_db::{Db, dual_test};

use masterdata_support::world;

/// A member's changes write no audit entry; an admin working inside a customer writes one
/// for every create, update and delete (same transaction); a refused change writes none.
async fn audit_of_changes(db: &Db) {
    let w = world(db).await;
    let pid = w.property(&w.a).await;
    w.unit(&w.a, &pid, 10).await;
    assert_eq!(audit_count(db, "property.create").await, 0);
    assert_eq!(audit_count(db, "unit.create").await, 0);

    w.admin_enters_a().await;
    let admin_pid = w.property(&w.admin).await;
    let uid = w.unit(&w.admin, &admin_pid, 10).await;
    let tid = w.tenant(&w.admin, &uid, json!({})).await;
    let body = json!({"street": "X", "zip": "1", "city": "Y", "totalShares": 100});
    w.expect(
        200,
        "PUT",
        &format!("/api/properties/{admin_pid}"),
        &w.admin,
        body,
    )
    .await;
    w.expect(
        400,
        "POST",
        "/api/units",
        &w.admin,
        json!({"propertyId": admin_pid, "name": "Z", "floor": "", "shares": 101}),
    )
    .await;
    w.expect(
        204,
        "DELETE",
        &format!("/api/tenants/{tid}"),
        &w.admin,
        serde_json::Value::Null,
    )
    .await;
    w.expect(
        404,
        "DELETE",
        &format!("/api/tenants/{tid}"),
        &w.admin,
        serde_json::Value::Null,
    )
    .await;

    assert_eq!(audit_count(db, "property.create").await, 1);
    assert_eq!(audit_count(db, "property.update").await, 1);
    assert_eq!(
        audit_count(db, "unit.create").await,
        1,
        "abgelehnte Einheit ohne Eintrag"
    );
    assert_eq!(audit_count(db, "tenant.create").await, 1);
    assert_eq!(audit_count(db, "tenant.delete").await, 1);
    // The admin sees the customer's data, a member of another customer does not.
    let list = w.app.get("/api/properties", Some(&w.admin)).await;
    assert_eq!(list.body.as_array().unwrap().len(), 2);
}
dual_test!(audit_member_vs_admin, audit_of_changes);

/// An admin without a customer has no master data (403 with a hint), without session 401.
async fn admin_without_account(db: &Db) {
    let w = world(db).await;
    let resp = w.app.get("/api/properties", Some(&w.admin)).await;
    assert_eq!(resp.status, 403);
    assert_eq!(w.app.get("/api/units", None).await.status, 401);
}
dual_test!(admin_needs_a_customer, admin_without_account);
