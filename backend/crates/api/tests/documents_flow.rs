//! Documents over the real router: upload with content check and size limit, storage
//! below the account's directory, download headers (RFC 6266, nosniff), delete with the
//! file, foreign ids (404) and the audit entry of an admin working inside a customer.

// Test bodies and helpers are plain async fns run by `dual_test!`.
#![allow(clippy::unwrap_used)]

mod documents_support;
mod masterdata_support;
mod support;

use serde_json::{Value, json};
use support::audit_count;
use vermieterme_db::{Db, dual_test};

use documents_support::{PDF, doc_world, entries, get_raw, period, send_raw, upload};

/// Upload, list, download, delete; another customer gets 404 everywhere.
async fn upload_download_delete(db: &Db) {
    let d = doc_world(db).await;
    let w = &d.w;
    let pid = w.property(&w.a).await;
    let uid = w.unit(&w.a, &pid, 40).await;
    let tid = w.tenant(&w.a, &uid, json!({})).await;
    let parts = [
        ("file", Some("Rechnung März.pdf"), PDF),
        ("category", None, &b"invoice"[..]),
        ("tenantId", None, tid.as_bytes()),
    ];
    let resp = upload(w, &w.a, &parts).await;
    assert_eq!(resp.status, 201, "{:?}", resp.json());
    let body = resp.json();
    let id = body["id"].as_str().unwrap().to_owned();
    assert_eq!(body["originalName"], "Rechnung März.pdf");
    assert_eq!(body["mimeType"], "application/pdf");
    assert_eq!(body["size"], PDF.len());
    assert_eq!(
        (&body["category"], &body["tenantId"]),
        (&json!("invoice"), &json!(tid))
    );
    assert_eq!(body["billingPeriodId"], Value::Null);
    assert!(body.get("fileName").is_none());

    // One stored file under a generated name in the account's directory, no leftovers.
    let account_dir = d.dir.join(&w.account_a);
    let stored = entries(&account_dir);
    assert_eq!(stored.len(), 1);
    assert!(stored[0].ends_with(".pdf") && !stored[0].contains("Rechnung"));
    assert_eq!(entries(&d.dir), vec![w.account_a.clone()]);

    let list = w
        .app
        .get(&format!("/api/documents?tenantId={tid}"), Some(&w.a))
        .await;
    assert_eq!(list.body[0]["id"], json!(id));
    assert_eq!(
        w.app.get("/api/documents", Some(&w.b)).await.body,
        json!([])
    );
    let foreign = w
        .app
        .get(&format!("/api/documents?tenantId={tid}"), Some(&w.b))
        .await;
    assert_eq!(foreign.status, 404);

    let file = get_raw(w, &format!("/api/documents/{id}/file"), &w.a).await;
    assert_eq!(file.status, 200);
    assert_eq!(file.body, PDF);
    assert_eq!(file.header("content-type"), Some("application/pdf"));
    assert_eq!(file.header("x-content-type-options"), Some("nosniff"));
    assert_eq!(
        file.header("content-disposition"),
        Some("inline; filename=\"Rechnung M_rz.pdf\"; filename*=UTF-8''Rechnung%20M%C3%A4rz.pdf")
    );
    let path = format!("/api/documents/{id}");
    assert_eq!(get_raw(w, &format!("{path}/file"), &w.b).await.status, 404);
    w.expect(404, "DELETE", &path, &w.b, Value::Null).await;
    assert_eq!(entries(&account_dir).len(), 1);

    w.expect(204, "DELETE", &path, &w.a, Value::Null).await;
    assert!(entries(&account_dir).is_empty());
    assert_eq!(get_raw(w, &format!("{path}/file"), &w.a).await.status, 404);
    w.expect(404, "DELETE", &path, &w.a, Value::Null).await;
}

dual_test!(upload_download_delete_cases, upload_download_delete);

/// The type comes from the content: a renamed HTML file is refused (400) whatever name
/// and type the client sends; PNG, JPEG and WebP pass. Nothing is left on disk.
async fn content_types(db: &Db) {
    let d = doc_world(db).await;
    let w = &d.w;
    let html = b"<html><script>alert(1)</script></html>";
    let resp = upload(w, &w.a, &[("file", Some("rechnung.pdf"), &html[..])]).await;
    assert_eq!(resp.status, 400);
    assert!(resp.json()["fields"]["file"].is_string());
    let empty = upload(w, &w.a, &[("file", Some("leer.pdf"), &b""[..])]).await;
    assert_eq!(empty.status, 400);
    assert!(entries(&d.dir.join(&w.account_a)).is_empty());

    let cases: [(&[u8], &str); 3] = [
        (b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR", "image/png"),
        (b"\xFF\xD8\xFF\xE0\0\x10JFIF\0", "image/jpeg"),
        (b"RIFF\x24\0\0\0WEBPVP8 ", "image/webp"),
    ];
    for (bytes, mime) in cases {
        let resp = upload(w, &w.a, &[("file", Some("bild.pdf"), bytes)]).await;
        assert_eq!(resp.status, 201, "{mime}");
        assert_eq!(resp.json()["mimeType"], mime);
        let id = resp.json()["id"].as_str().unwrap().to_owned();
        let file = get_raw(w, &format!("/api/documents/{id}/file"), &w.a).await;
        assert_eq!(file.header("content-type"), Some(mime));
    }

    let missing = upload(w, &w.a, &[("category", None, &b"other"[..])]).await;
    assert_eq!(missing.status, 400);
    let bad_category = upload(
        w,
        &w.a,
        &[("file", Some("a.pdf"), PDF), ("category", None, &b"x"[..])],
    )
    .await;
    assert_eq!(bad_category.status, 400);
    assert!(bad_category.json()["fields"]["category"].is_string());
    let two = upload(
        w,
        &w.a,
        &[("file", Some("a.pdf"), PDF), ("file", Some("b.pdf"), PDF)],
    )
    .await;
    assert_eq!(two.status, 400);
    let not_form = send_raw(
        w,
        "POST",
        "/api/documents",
        &w.a,
        Some("application/json"),
        b"{}".to_vec(),
    )
    .await;
    assert_eq!(not_form.status, 400);
    assert_eq!(entries(&d.dir.join(&w.account_a)).len(), 3);
}

dual_test!(content_types_cases, content_types);

/// More than 10 MB is 413 `too_large`, nothing is stored; exactly 10 MB passes.
async fn size_limit(db: &Db) {
    let d = doc_world(db).await;
    let w = &d.w;
    let mut big = PDF.to_vec();
    big.resize(10 * 1024 * 1024 + 1, b' ');
    let resp = upload(w, &w.a, &[("file", Some("gross.pdf"), &big)]).await;
    assert_eq!(resp.status, 413);
    assert_eq!(resp.json()["code"], "too_large");
    assert!(entries(&d.dir.join(&w.account_a)).is_empty());
    big.truncate(10 * 1024 * 1024);
    let resp = upload(w, &w.a, &[("file", Some("genau.pdf"), &big)]).await;
    assert_eq!(resp.status, 201);
    let mut huge = PDF.to_vec();
    huge.resize(12 * 1024 * 1024, b' ');
    let resp = upload(w, &w.a, &[("file", Some("riesig.pdf"), &huge)]).await;
    assert_eq!(resp.status, 413);
    assert_eq!(entries(&d.dir.join(&w.account_a)).len(), 1);
}

dual_test!(size_limit_cases, size_limit);

/// Assignments of another customer are 404 (nothing stored); an admin's upload inside a
/// customer writes one audit entry, a member's none.
async fn assignments_and_audit(db: &Db) {
    let d = doc_world(db).await;
    let w = &d.w;
    let pid = w.property(&w.a).await;
    let uid = w.unit(&w.a, &pid, 40).await;
    let tid = w.tenant(&w.a, &uid, json!({})).await;
    let period = period(w, &w.a, &pid).await;
    for (field, id) in [("tenantId", &tid), ("billingPeriodId", &period)] {
        let resp = upload(
            w,
            &w.b,
            &[("file", Some("a.pdf"), PDF), (field, None, id.as_bytes())],
        )
        .await;
        assert_eq!(resp.status, 404, "{field}");
    }
    // Account B's directory may exist, but holds no file.
    let files = entries(&d.dir)
        .iter()
        .flat_map(|a| entries(&d.dir.join(a)))
        .count();
    assert_eq!(files, 0);
    let ok = upload(
        w,
        &w.a,
        &[
            ("file", Some("a.pdf"), PDF),
            ("billingPeriodId", None, period.as_bytes()),
        ],
    )
    .await;
    assert_eq!(ok.status, 201);
    assert_eq!(ok.json()["billingPeriodId"], json!(period));
    assert_eq!(audit_count(db, "document.create").await, 0);

    w.admin_enters_a().await;
    let resp = upload(w, &w.admin, &[("file", Some("a.pdf"), PDF)]).await;
    assert_eq!(resp.status, 201);
    assert_eq!(audit_count(db, "document.create").await, 1);
    let id = resp.json()["id"].as_str().unwrap().to_owned();
    w.expect(
        204,
        "DELETE",
        &format!("/api/documents/{id}"),
        &w.admin,
        Value::Null,
    )
    .await;
    assert_eq!(audit_count(db, "document.delete").await, 1);
}

dual_test!(assignments_and_audit_cases, assignments_and_audit);

/// A row whose file vanished is 404 on download, not 500.
async fn missing_file(db: &Db) {
    let d = doc_world(db).await;
    let w = &d.w;
    let resp = upload(w, &w.a, &[("file", Some("a.pdf"), PDF)]).await;
    let id = resp.json()["id"].as_str().unwrap().to_owned();
    let dir = d.dir.join(&w.account_a);
    for name in entries(&dir) {
        std::fs::remove_file(dir.join(name)).unwrap();
    }
    let file = get_raw(w, &format!("/api/documents/{id}/file"), &w.a).await;
    assert_eq!(file.status, 404);
    assert_eq!(file.json()["error"], "Datei nicht gefunden");
}

dual_test!(missing_file_cases, missing_file);
