//! Tests of the error shape.

use axum::body::to_bytes;
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use serde_json::{Value, json};
use std::time::Duration;
use vermieterme_db::DbError;

use super::ApiError;

/// Status, headers and JSON body of an error response.
async fn render(error: ApiError) -> (StatusCode, axum::http::HeaderMap, Value) {
    let response = error.into_response();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), 1 << 16).await.unwrap();
    (status, headers, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn csrf_and_internal_use_contract_texts() {
    let (status, _, body) = render(ApiError::csrf()).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, json!({"error": "Anfrage abgelehnt", "code": "csrf"}));
    let (status, _, body) = render(ApiError::internal()).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        body,
        json!({"error": "Interner Fehler", "code": "internal"})
    );
}

#[tokio::test]
async fn validation_lists_fields() {
    let error = ApiError::validation(vec![
        ("name".into(), "Name fehlt".into()),
        ("email".into(), "E-Mail-Adresse ist ungültig".into()),
    ]);
    let (status, _, body) = render(error).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "Name fehlt");
    assert_eq!(body["code"], "validation");
    assert_eq!(body["fields"]["email"], "E-Mail-Adresse ist ungültig");
}

#[tokio::test]
async fn rate_limit_sets_retry_after() {
    let (status, headers, body) = render(ApiError::rate_limited(Duration::from_millis(1500))).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(headers[header::RETRY_AFTER], "2");
    assert_eq!(body["code"], "rate_limited");
}

#[tokio::test]
async fn database_errors_map_to_contract_statuses() {
    assert_eq!(
        ApiError::from(DbError::NotFound).status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        ApiError::from(DbError::Conflict).status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        ApiError::from(DbError::Constraint).status(),
        StatusCode::BAD_REQUEST
    );
    let internal = ApiError::from(DbError::InvalidValue("app_user.role"));
    assert_eq!(internal.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(internal.message(), "Interner Fehler");
}
