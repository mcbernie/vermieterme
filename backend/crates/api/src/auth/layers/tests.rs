//! Unit tests of the CSRF rules (the router-level behaviour is covered by the
//! integration tests).

use super::*;

fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
    let mut h = HeaderMap::new();
    for (k, v) in pairs {
        h.insert(*k, v.parse().unwrap());
    }
    h
}

#[test]
fn only_mutating_web_api_requests_need_the_guard() {
    assert!(needs_guard(&Method::POST, "/api/auth/login"));
    assert!(needs_guard(&Method::DELETE, "/api/users/1"));
    assert!(!needs_guard(&Method::GET, "/api/users"));
    assert!(!needs_guard(&Method::POST, "/api/tenant-app/auth"));
    assert!(!needs_guard(&Method::POST, "/upload"));
    assert!(needs_guard(&Method::POST, "/api/tenant-appx"));
}

#[test]
fn guard_header_must_be_exactly_one() {
    assert!(guard_ok(&headers(&[("x-csrf-guard", "1")]), false));
    assert!(!guard_ok(&headers(&[("x-csrf-guard", "true")]), false));
    assert!(!guard_ok(&HeaderMap::new(), false));
}

#[test]
fn origin_must_match_own_host() {
    let same = headers(&[
        ("x-csrf-guard", "1"),
        ("origin", "https://app.example"),
        ("host", "app.example"),
    ]);
    assert!(guard_ok(&same, false));
    let foreign = headers(&[
        ("x-csrf-guard", "1"),
        ("origin", "https://evil.example"),
        ("host", "app.example"),
    ]);
    assert!(!guard_ok(&foreign, false));
    let null = headers(&[
        ("x-csrf-guard", "1"),
        ("origin", "null"),
        ("host", "app.example"),
    ]);
    assert!(!guard_ok(&null, false));
}

#[test]
fn forwarded_host_counts_only_behind_trusted_proxy() {
    let h = headers(&[
        ("x-csrf-guard", "1"),
        ("origin", "https://app.example"),
        ("host", "127.0.0.1:3000"),
        ("x-forwarded-host", "app.example"),
    ]);
    assert!(!guard_ok(&h, false));
    assert!(guard_ok(&h, true));
}

#[test]
fn origin_host_parsing() {
    assert_eq!(
        host_of_origin("http://localhost:3000"),
        Some("localhost:3000")
    );
    assert_eq!(host_of_origin("https://a.example/path"), None);
    assert_eq!(host_of_origin("ftp://a.example"), None);
    assert_eq!(host_of_origin("null"), None);
}
