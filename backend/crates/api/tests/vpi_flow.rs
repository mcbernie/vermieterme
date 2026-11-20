//! VPI values (global, admin writes), the retrieval from a local stand-in for the
//! Bundesbank (no network) and the rent proposals.

// Test bodies and helpers are plain async fns run by `dual_test!`.
#![allow(clippy::unwrap_used)]

mod masterdata_support;
mod support;

use std::net::SocketAddr;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
use serde_json::{Value, json};
use support::{TestApp, audit_count, config, seed_account, seed_user};
use tokio::net::TcpListener;
use vermieterme_db::repo::user::Role;
use vermieterme_db::{Db, dual_test};

use masterdata_support::{id_of, world};

/// SDMX-JSON of three months (2024-01 .. 2024-03) as the Bundesbank delivers it.
const SERIES: &str = r#"{"data":{"structure":{"dimensions":{"observation":[{"values":[
    {"id":"2024-01"},{"id":"2024-02"},{"id":"2024-03"}]}]}},
    "dataSets":[{"series":{"0:0:0:0:0:0:0:0":{"observations":{
    "0":["117.6"],"1":["118.1"],"2":["118.65"]}}}}]}}"#;

/// Starts a stand-in Bundesbank that answers every request with `status` and `body`.
async fn stand_in(status: StatusCode, body: &'static str) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = Router::new().fallback(get(move || async move { (status, body) }));
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    addr
}

/// A server whose `VPI_API_BASE` points at `addr`, with the same accounts as `world`.
async fn app_with_base(db: &Db, addr: SocketAddr) -> (TestApp, String) {
    let base = format!("http://{addr}/rest/data");
    let app = TestApp::start_with(db, config(&[("VPI_API_BASE", &base)])).await;
    let account = seed_account(db, &format!("Kunde {}", addr.port())).await;
    let email = format!("m{}@example.org", addr.port());
    seed_user(db, Some(&account.id), &email, Role::Member).await;
    let cookie = app.login(&email, support::PASSWORD).await;
    (app, cookie)
}

/// Members read, only admins write; upsert on the same month and base year; audit entries;
/// validation; unknown id -> 404.
async fn values(db: &Db) {
    let w = world(db).await;
    let body =
        |tenths: i64| json!({"year": 2025, "month": 5, "valueTenths": tenths, "baseYear": 2020});
    w.expect(403, "POST", "/api/vpi", &w.a, body(1196)).await;
    let first = w
        .expect(201, "POST", "/api/vpi", &w.admin, body(1196))
        .await;
    let second = w
        .expect(201, "POST", "/api/vpi", &w.admin, body(1200))
        .await;
    assert_eq!(
        id_of(&first),
        id_of(&second),
        "gleicher Monat und Basisjahr wird aktualisiert"
    );
    assert_eq!(audit_count(db, "vpi.save").await, 2);

    let list = w.app.get("/api/vpi", Some(&w.b)).await;
    assert_eq!(list.body.as_array().unwrap().len(), 1);
    assert_eq!(list.body[0]["valueTenths"], 1200);
    assert_eq!(w.app.get("/api/vpi", None).await.status, 401);

    for (patch, field) in [
        (json!({"month": 13}), "month"),
        (json!({"year": 1500}), "year"),
        (json!({"valueTenths": 0}), "valueTenths"),
        (json!({"baseYear": 2005}), "baseYear"),
    ] {
        let mut bad = body(1000);
        bad.as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        let resp = w.expect(400, "POST", "/api/vpi", &w.admin, bad).await;
        assert!(resp.body["fields"][field].is_string(), "{field}");
    }

    let path = format!("/api/vpi/{}", id_of(&first));
    w.expect(403, "DELETE", &path, &w.a, Value::Null).await;
    w.expect(204, "DELETE", &path, &w.admin, Value::Null).await;
    w.expect(404, "DELETE", &path, &w.admin, Value::Null).await;
    assert_eq!(audit_count(db, "vpi.delete").await, 1);
}
dual_test!(vpi_values_admin_writes, values);

/// The retrieval returns the parsed series without storing it; errors of the service
/// give 502 `upstream`, an empty answer 404, unsupported base years 400.
async fn retrieval(db: &Db) {
    let ok = stand_in(StatusCode::OK, SERIES).await;
    let (app, cookie) = app_with_base(db, ok).await;
    let resp = app
        .call(
            "POST",
            "/api/vpi/fetch",
            Some(&cookie),
            json!({"baseYear": 2020}),
        )
        .await;
    assert_eq!(resp.status, 200, "{:?}", resp.body);
    assert_eq!(resp.body["baseYear"], 2020);
    assert_eq!(
        resp.body["entries"][0],
        json!({"year": 2024, "month": 1, "valueTenths": 1176})
    );
    assert_eq!(
        resp.body["entries"][2]["valueTenths"], 1187,
        "gerundet auf eine Nachkommastelle"
    );
    assert_eq!(
        app.get("/api/vpi", Some(&cookie)).await.body,
        json!([]),
        "nichts gespeichert"
    );

    let unsupported = app
        .call(
            "POST",
            "/api/vpi/fetch",
            Some(&cookie),
            json!({"baseYear": 2005}),
        )
        .await;
    assert_eq!(unsupported.status, 400);
    assert!(unsupported.body["fields"]["baseYear"].is_string());
    let missing = app
        .call("POST", "/api/vpi/fetch", Some(&cookie), json!({}))
        .await;
    assert_eq!(missing.status, 400);
    let anonymous = app
        .call("POST", "/api/vpi/fetch", None, json!({"baseYear": 2020}))
        .await;
    assert_eq!(anonymous.status, 401);
}
dual_test!(vpi_fetch_parses_series, retrieval);

/// Failures of the service: HTTP error, garbage, empty series, unreachable.
async fn retrieval_failures(db: &Db) {
    let empty = r#"{"data":{"structure":{"dimensions":{"observation":[]}},"dataSets":[]}}"#;
    let cases = [
        (stand_in(StatusCode::INTERNAL_SERVER_ERROR, "{}").await, 502),
        (stand_in(StatusCode::OK, "kein json").await, 502),
        (stand_in(StatusCode::OK, empty).await, 404),
    ];
    for (addr, status) in cases {
        let (app, cookie) = app_with_base(db, addr).await;
        let request = json!({"baseYear": 2015});
        let resp = app
            .call("POST", "/api/vpi/fetch", Some(&cookie), request)
            .await;
        assert_eq!(resp.status, status, "{:?}", resp.body);
        if status == 502 {
            assert_eq!(resp.body["code"], "upstream");
            assert!(resp.body["error"].as_str().unwrap().contains("manuell"));
        }
    }
    // Nothing listens on this port any more: connection refused is also 502.
    let closed = {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        listener.local_addr().unwrap()
    };
    let (app, cookie) = app_with_base(db, closed).await;
    let resp = app
        .call(
            "POST",
            "/api/vpi/fetch",
            Some(&cookie),
            json!({"baseYear": 2010}),
        )
        .await;
    assert_eq!(resp.status, 502);
}
dual_test!(vpi_fetch_failures, retrieval_failures);

/// Proposals: only active index tenants of the own customer with a rent and an index value
/// of their base year; the calculation is the domain's.
async fn suggestions(db: &Db) {
    let w = world(db).await;
    let pid = w.property(&w.a).await;
    let uid = w.unit(&w.a, &pid, 10).await;
    let index = json!({"leaseType": "index", "indexBaseYear": 2020,
        "indexReferenceValueTenths": 1000, "indexReferenceDate": "2020-01-01", "indexMinMonths": 12});
    let tid = w.tenant(&w.a, &uid, index).await;
    let other = w.unit(&w.a, &pid, 10).await;
    w.tenant(&w.a, &other, json!({"leaseType": "standard"}))
        .await;
    let rent = |amount: i64, date: &str| json!({"unitId": uid, "type": "rent", "amountCents": amount, "effectiveDate": date});
    w.expect(
        201,
        "POST",
        "/api/rent-changes",
        &w.a,
        rent(60_000, "2020-01-01"),
    )
    .await;
    w.expect(
        201,
        "POST",
        "/api/rent-changes",
        &w.a,
        rent(65_000, "2022-01-01"),
    )
    .await;
    w.expect(201, "POST", "/api/rent-changes", &w.a, json!({"unitId": uid, "type": "prepayment", "amountCents": 99_000, "effectiveDate": "2023-01-01"})).await;

    assert_eq!(
        w.app.get("/api/vpi/suggestions", Some(&w.a)).await.body,
        json!([]),
        "ohne Indexwert"
    );
    let value = json!({"year": 2024, "month": 12, "valueTenths": 1100, "baseYear": 2020});
    w.expect(201, "POST", "/api/vpi", &w.admin, value).await;

    let resp = w.app.get("/api/vpi/suggestions", Some(&w.a)).await;
    let items = resp.body.as_array().unwrap();
    assert_eq!(items.len(), 1, "{:?}", resp.body);
    let s = &items[0];
    assert_eq!(s["tenantId"], tid.as_str());
    assert_eq!(s["currentRentCents"], 65_000);
    assert_eq!(s["referenceVpiTenths"], 1000);
    assert_eq!(s["currentVpiTenths"], 1100);
    assert_eq!(s["percentageChangeBasisPoints"], 1000);
    assert_eq!(s["suggestedRentCents"], 71_500);
    assert_eq!(s["eligible"], true);
    assert_eq!(s["leaseType"], "index");
    assert_eq!(s["baseYear"], 2020);
    assert_eq!(s["propertyAddress"], "Musterweg 1, Beispielstadt");
    assert_eq!(s["tenantName"], "Erika Muster");
    assert_eq!(
        w.app.get("/api/vpi/suggestions", Some(&w.b)).await.body,
        json!([])
    );
    assert_eq!(w.app.get("/api/vpi/suggestions", None).await.status, 401);
}
dual_test!(vpi_suggestions_scoped, suggestions);

/// Global VPI changes carry no customer id in the audit log, even if the admin works
/// inside a customer.
async fn global_audit(db: &Db) {
    use sqlx::Row as _;
    let w = world(db).await;
    w.admin_enters_a().await;
    let body = json!({"year": 2025, "month": 5, "valueTenths": 1196, "baseYear": 2020});
    let saved = w.expect(201, "POST", "/api/vpi", &w.admin, body).await;
    let path = format!("/api/vpi/{}", id_of(&saved));
    w.expect(204, "DELETE", &path, &w.admin, Value::Null).await;
    let mut conn = db.pool().acquire().await.unwrap();
    let row = sqlx::query(
        "SELECT COUNT(*) AS n FROM audit_log WHERE entity = 'vpi_entry' AND account_id IS NULL",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    assert_eq!(row.get::<i64, _>("n"), 2);
}
dual_test!(vpi_audit_is_global, global_audit);

/// The retrieval is limited to one per minute for the whole system: the second one is 429
/// with `Retry-After`, also for another customer; invalid requests do not count.
async fn retrieval_limit(db: &Db) {
    let ok = stand_in(StatusCode::OK, SERIES).await;
    let (app, cookie) = app_with_base(db, ok).await;
    let account = seed_account(db, "Zweiter Kunde").await;
    seed_user(db, Some(&account.id), "zweiter@example.org", Role::Member).await;
    let other = app.login("zweiter@example.org", support::PASSWORD).await;
    let request = json!({"baseYear": 2020});
    let invalid = json!({"baseYear": 1999});
    let call = async |cookie: &str, body: Value| {
        app.call("POST", "/api/vpi/fetch", Some(cookie), body).await
    };

    assert_eq!(call(&cookie, invalid).await.status, 400);
    assert_eq!(call(&cookie, request.clone()).await.status, 200);
    let limited = call(&cookie, request.clone()).await;
    assert_eq!(limited.status, 429);
    assert_eq!(limited.body["code"], "rate_limited");
    assert!(limited.header("retry-after").is_some());
    assert_eq!(call(&other, request).await.status, 429);
}
dual_test!(vpi_fetch_limited, retrieval_limit);
