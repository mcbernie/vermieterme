//! Shared set-up of the document tests: a world with its own upload directory, multipart
//! uploads and raw (binary) responses, which the JSON client of `support` cannot carry.

#![allow(dead_code)] // Not every test binary uses every helper.
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;
use vermieterme_db::Db;

use super::masterdata_support::{World, id_of, world_with};
use super::support::config;

/// Boundary of the multipart bodies.
pub const BOUNDARY: &str = "----vermieterme-test-boundary";

/// A minimal PDF head (enough for the content check).
pub const PDF: &[u8] = b"%PDF-1.7\n1 0 obj << >> endobj\ntrailer << >>\n%%EOF\n";

/// [`World`] plus its upload directory (removed on drop).
pub struct DocWorld {
    /// Accounts, sessions, server.
    pub w: World,
    /// `UPLOAD_DIR` of the server.
    pub dir: PathBuf,
}

impl Drop for DocWorld {
    /// Removes the upload directory of the test.
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Starts a world whose `UPLOAD_DIR` is a fresh temporary directory.
pub async fn doc_world(db: &Db) -> DocWorld {
    doc_world_with(db, &[]).await
}

/// Like [`doc_world`] with further configuration `pairs`.
pub async fn doc_world_with(db: &Db, pairs: &[(&str, &str)]) -> DocWorld {
    let dir = std::env::temp_dir().join(format!(
        "vermieterme-uploads-{}",
        vermieterme_db::row::new_id()
    ));
    let mut all = vec![("UPLOAD_DIR", dir.to_str().unwrap())];
    all.extend_from_slice(pairs);
    let w = world_with(db, config(&all)).await;
    DocWorld { w, dir }
}

/// A raw response.
#[derive(Debug)]
pub struct Raw {
    /// Status code.
    pub status: u16,
    /// Headers (lower-case names).
    pub headers: Vec<(String, String)>,
    /// Body bytes.
    pub body: Vec<u8>,
}

impl Raw {
    /// First header of `name`.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// The body as JSON (`Null` if it is none).
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }
}

/// One part of a multipart body: `(name, file name, bytes)`.
pub type Part<'a> = (&'a str, Option<&'a str>, &'a [u8]);

/// Builds a `multipart/form-data` body.
pub fn multipart(parts: &[Part<'_>]) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, file_name, bytes) in parts {
        body.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
        let disposition = match file_name {
            Some(f) => format!(
                "Content-Disposition: form-data; name=\"{name}\"; filename=\"{f}\"\r\n\
                 Content-Type: application/pdf\r\n"
            ),
            None => format!("Content-Disposition: form-data; name=\"{name}\"\r\n"),
        };
        body.extend_from_slice(disposition.as_bytes());
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    body
}

/// Sends a request with a raw body and returns the raw response. Writes and reads at the
/// same time, so a server that answers before reading the whole body (413) is seen.
pub async fn send_raw(
    w: &World,
    method: &str,
    path: &str,
    cookie: &str,
    content_type: Option<&str>,
    body: Vec<u8>,
) -> Raw {
    let mut head = format!(
        "{method} {path} HTTP/1.0\r\nHost: {}\r\nCookie: {cookie}\r\nX-CSRF-Guard: 1\r\n\
         Content-Length: {}\r\n",
        w.app.addr,
        body.len()
    );
    if let Some(ct) = content_type {
        head.push_str(&format!("Content-Type: {ct}\r\n"));
    }
    head.push_str("\r\n");
    let stream = TcpStream::connect(w.app.addr).await.unwrap();
    let (mut reader, mut writer) = stream.into_split();
    let write = tokio::spawn(async move {
        let _ = writer.write_all(head.as_bytes()).await;
        let _ = writer.write_all(&body).await;
        writer
    });
    let mut raw = Vec::new();
    let _ = reader.read_to_end(&mut raw).await;
    drop(write.await);
    parse(&raw)
}

/// Uploads `parts` as `cookie`.
pub async fn upload(w: &World, cookie: &str, parts: &[Part<'_>]) -> Raw {
    let content_type = format!("multipart/form-data; boundary={BOUNDARY}");
    let body = multipart(parts);
    send_raw(
        w,
        "POST",
        "/api/documents",
        cookie,
        Some(&content_type),
        body,
    )
    .await
}

/// `GET` as `cookie`, raw.
pub async fn get_raw(w: &World, path: &str, cookie: &str) -> Raw {
    let head = format!(
        "GET {path} HTTP/1.0\r\nHost: {}\r\nCookie: {cookie}\r\n\r\n",
        w.app.addr
    );
    let mut stream = TcpStream::connect(w.app.addr).await.unwrap();
    stream.write_all(head.as_bytes()).await.unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.unwrap();
    parse(&raw)
}

/// Parses a full HTTP/1.x response with a binary body.
fn parse(raw: &[u8]) -> Raw {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or_else(|| panic!("no response head: {} bytes", raw.len()));
    let head = String::from_utf8_lossy(&raw[..split]);
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .unwrap()
        .split(' ')
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let headers = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_lowercase(), v.trim().to_owned()))
        .collect();
    Raw {
        status,
        headers,
        body: raw[split + 4..].to_vec(),
    }
}

/// Creates a billing period 2024 of the property; returns its id.
pub async fn period(w: &World, cookie: &str, pid: &str) -> String {
    let body = json!({"propertyId": pid, "startDate": "2024-01-01", "endDate": "2024-12-31"});
    id_of(
        &w.expect(201, "POST", "/api/billing-periods", cookie, body)
            .await,
    )
}

/// Uploads a PDF with the given text fields; returns the new document's id.
pub async fn upload_pdf(w: &World, cookie: &str, fields: &[(&str, &str)]) -> String {
    let mut parts: Vec<Part<'_>> = vec![("file", Some("a.pdf"), PDF)];
    parts.extend(fields.iter().map(|(k, v)| (*k, None, v.as_bytes())));
    let resp = upload(w, cookie, &parts).await;
    assert_eq!(resp.status, 201, "{:?}", resp.json());
    resp.json()["id"].as_str().unwrap().to_owned()
}

/// Names of all entries (files, also temporary ones) in a directory; empty if missing.
pub fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|it| {
            it.filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}
