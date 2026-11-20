//! Resource limits over the real router: storage quota and upload rate per account
//! (decision 44, #124) and the time limit of requests (decision 45).

// Test bodies and helpers are plain async fns run by `dual_test!`.
#![allow(clippy::unwrap_used)]

mod documents_support;
mod masterdata_support;
mod support;

use std::time::Duration;

use serde_json::Value;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;
use vermieterme_db::{Db, dual_test};

use documents_support::{BOUNDARY, PDF, doc_world_with, entries, multipart, upload};
use masterdata_support::World;

/// A PDF of `size` bytes (head plus padding).
fn pdf_of(size: usize) -> Vec<u8> {
    let mut bytes = PDF.to_vec();
    bytes.resize(size, b' ');
    bytes
}

/// Uploads a PDF of `size` bytes as `cookie`; returns status and JSON body.
async fn upload_sized(w: &World, cookie: &str, size: usize) -> (u16, Value) {
    let file = pdf_of(size);
    let resp = upload(w, cookie, &[("file", Some("a.pdf"), &file)]).await;
    (resp.status, resp.json())
}

/// Over the quota: 413 `quota_exceeded`, no file left; deleting frees the space; the
/// other account is not affected.
async fn quota(db: &Db) {
    let d = doc_world_with(db, &[("UPLOAD_QUOTA_BYTES", "100000")]).await;
    let w = &d.w;
    let (status, first) = upload_sized(w, &w.a, 60_000).await;
    assert_eq!(status, 201, "{first:?}");
    let (status, body) = upload_sized(w, &w.a, 60_000).await;
    assert_eq!(status, 413);
    assert_eq!(body["code"], "quota_exceeded");
    assert_eq!(
        entries(&d.dir.join(&w.account_a)).len(),
        1,
        "keine Datei übrig"
    );
    assert_eq!(upload_sized(w, &w.b, 60_000).await.0, 201);

    let path = format!("/api/documents/{}", first["id"].as_str().unwrap());
    w.expect(204, "DELETE", &path, &w.a, Value::Null).await;
    assert_eq!(upload_sized(w, &w.a, 60_000).await.0, 201);
}
dual_test!(quota_cases, quota);

/// The 61st upload of an account within an hour is 429 with `Retry-After`, refused ones
/// count too; another account still uploads.
async fn upload_rate(db: &Db) {
    let d = doc_world_with(db, &[]).await;
    let w = &d.w;
    for n in 0..60 {
        let file: &[u8] = if n == 0 { b"kein pdf" } else { PDF };
        let resp = upload(w, &w.a, &[("file", Some("a.pdf"), file)]).await;
        assert_eq!(resp.status, if n == 0 { 400 } else { 201 }, "Upload {n}");
    }
    let resp = upload(w, &w.a, &[("file", Some("a.pdf"), PDF)]).await;
    assert_eq!(resp.status, 429);
    assert_eq!(resp.json()["code"], "rate_limited");
    assert!(resp.header("retry-after").is_some());
    assert_eq!(
        upload(w, &w.b, &[("file", Some("a.pdf"), PDF)])
            .await
            .status,
        201
    );
}
dual_test!(upload_rate_cases, upload_rate);

/// Sends `head` and `parts` of a request with pauses of `pause` between the parts;
/// returns status and JSON body.
async fn send_slowly(
    w: &World,
    head: String,
    parts: Vec<Vec<u8>>,
    pause: Duration,
) -> (u16, Value) {
    let stream = TcpStream::connect(w.app.addr).await.unwrap();
    let (mut reader, mut writer) = stream.into_split();
    let write = tokio::spawn(async move {
        let _ = writer.write_all(head.as_bytes()).await;
        for part in parts {
            tokio::time::sleep(pause).await;
            let _ = writer.write_all(&part).await;
        }
        writer
    });
    let mut raw = Vec::new();
    tokio::time::timeout(Duration::from_secs(10), reader.read_to_end(&mut raw))
        .await
        .unwrap()
        .unwrap();
    write.abort();
    let text = String::from_utf8_lossy(&raw);
    let status = text.split(' ').nth(1).unwrap().parse().unwrap();
    let body = text.split("\r\n\r\n").nth(1).unwrap_or_default();
    (status, serde_json::from_str(body).unwrap_or(Value::Null))
}

/// A request head with `Content-Length`.
fn head(w: &World, path: &str, content_type: &str, length: usize) -> String {
    format!(
        "POST {path} HTTP/1.0\r\nHost: {}\r\nCookie: {}\r\nX-CSRF-Guard: 1\r\n\
         Content-Type: {content_type}\r\nContent-Length: {length}\r\n\r\n",
        w.app.addr, w.a
    )
}

/// With a limit of 1 s, a request whose body stalls gets 503 `timeout` in the error
/// shape; the upload keeps its own, longer limit.
async fn time_limit(db: &Db) {
    let d = doc_world_with(db, &[("REQUEST_TIMEOUT_SECS", "1")]).await;
    let w = &d.w;
    let json = br#"{"street":"Musterweg 1"}"#.to_vec();
    let stalled = head(w, "/api/properties", "application/json", json.len());
    let (status, body) = send_slowly(w, stalled, vec![json], Duration::from_secs(3)).await;
    assert_eq!(status, 503);
    assert_eq!(body["code"], "timeout");

    let form = multipart(&[("file", Some("a.pdf"), PDF)]);
    let (first, rest) = form.split_at(form.len() / 2);
    let content_type = format!("multipart/form-data; boundary={BOUNDARY}");
    let slow = head(w, "/api/documents", &content_type, form.len());
    let parts = vec![first.to_vec(), rest.to_vec()];
    let (status, body) = send_slowly(w, slow, parts, Duration::from_millis(800)).await;
    assert_eq!(status, 201, "{body:?}");
}
dual_test!(time_limit_cases, time_limit);
