//! Cascading deletes remove the stored files of the documents they take along (#106):
//! property, unit, tenant and billing period. Documents outside the cascade keep theirs.

// Test bodies and helpers are plain async fns run by `dual_test!`.
#![allow(clippy::unwrap_used)]

mod documents_support;
mod masterdata_support;
mod support;

use serde_json::{Value, json};
use vermieterme_db::{Db, dual_test};

use documents_support::{doc_world, entries, period, upload_pdf};

/// Each delete removes exactly the files of its cascade; the rest stays readable.
async fn cascades_remove_files(db: &Db) {
    let d = doc_world(db).await;
    let w = &d.w;
    let dir = d.dir.join(&w.account_a);
    let pid = w.property(&w.a).await;
    let u1 = w.unit(&w.a, &pid, 40).await;
    let u2 = w.unit(&w.a, &pid, 40).await;
    let t1 = w.tenant(&w.a, &u1, json!({})).await;
    let t2 = w.tenant(&w.a, &u2, json!({})).await;
    let t3 = w.tenant(&w.a, &u2, json!({})).await;
    let bp = period(w, &w.a, &pid).await;
    upload_pdf(w, &w.a, &[("tenantId", &t1)]).await;
    upload_pdf(w, &w.a, &[("tenantId", &t2)]).await;
    upload_pdf(w, &w.a, &[("tenantId", &t3)]).await;
    upload_pdf(w, &w.a, &[("billingPeriodId", &bp)]).await;
    upload_pdf(w, &w.a, &[("billingPeriodId", &bp), ("tenantId", &t3)]).await;
    let loose = upload_pdf(w, &w.a, &[]).await;
    let other_pid = w.property(&w.a).await;
    let other_unit = w.unit(&w.a, &other_pid, 10).await;
    let other_tenant = w.tenant(&w.a, &other_unit, json!({})).await;
    upload_pdf(w, &w.a, &[("tenantId", &other_tenant)]).await;
    assert_eq!(entries(&dir).len(), 7);

    let delete = async |path: String| {
        w.expect(204, "DELETE", &path, &w.a, Value::Null).await;
    };
    delete(format!("/api/tenants/{t1}")).await;
    assert_eq!(entries(&dir).len(), 6);
    delete(format!("/api/billing-periods/{bp}")).await;
    assert_eq!(entries(&dir).len(), 4);
    delete(format!("/api/units/{u2}")).await;
    assert_eq!(entries(&dir).len(), 2);
    delete(format!("/api/properties/{other_pid}")).await;
    assert_eq!(entries(&dir).len(), 1);

    let list = w.app.get("/api/documents", Some(&w.a)).await;
    assert_eq!(list.body[0]["id"], json!(loose));
    assert_eq!(list.body.as_array().unwrap().len(), 1);
}

dual_test!(cascades_remove_files_cases, cascades_remove_files);

/// Deleting a property with documents of its periods and tenants leaves no file behind;
/// another customer cannot delete it (404) and the files stay.
async fn property_delete_leaves_no_orphans(db: &Db) {
    let d = doc_world(db).await;
    let w = &d.w;
    let pid = w.property(&w.a).await;
    let uid = w.unit(&w.a, &pid, 40).await;
    let tid = w.tenant(&w.a, &uid, json!({})).await;
    let bp = period(w, &w.a, &pid).await;
    upload_pdf(w, &w.a, &[("tenantId", &tid)]).await;
    upload_pdf(w, &w.a, &[("billingPeriodId", &bp)]).await;
    upload_pdf(w, &w.a, &[("billingPeriodId", &bp), ("tenantId", &tid)]).await;
    let dir = d.dir.join(&w.account_a);
    assert_eq!(entries(&dir).len(), 3);
    let path = format!("/api/properties/{pid}");
    w.expect(404, "DELETE", &path, &w.b, Value::Null).await;
    assert_eq!(entries(&dir).len(), 3);
    w.expect(204, "DELETE", &path, &w.a, Value::Null).await;
    assert!(entries(&dir).is_empty());
    assert_eq!(
        w.app.get("/api/documents", Some(&w.a)).await.body,
        json!([])
    );
}

dual_test!(property_delete_cases, property_delete_leaves_no_orphans);
