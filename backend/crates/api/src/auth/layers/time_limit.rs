//! Time limit of every request (decision 45): `REQUEST_TIMEOUT_SECS`, the upload route
//! [`UPLOAD_TIMEOUT`]. A request past its limit is dropped and answered with 503 `timeout`
//! in the contract's error shape.
//!
//! A small middleware instead of `tower_http::timeout::TimeoutLayer`: that layer has one
//! fixed duration and answers with an empty body, while here the limit depends on the
//! route and the answer must carry the error shape. Dropping the handler is safe: work
//! that must outlive the request (commit of an upload or delete) runs detached
//! (`documents::detached`); a partly written upload file is removed by its guard.
//! Slow clients that never finish their headers are the reverse proxy's job (plan).

use axum::extract::{Request, State};
use axum::http::Method;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::time::Duration;

use crate::config::UPLOAD_TIMEOUT;
use crate::error::ApiError;
use crate::routes::AppState;

/// Runs the request within its time limit, else 503 `timeout`.
pub(super) async fn time_limit(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let limit = limit_for(req.method(), req.uri().path(), state.config.request_timeout);
    match tokio::time::timeout(limit, next.run(req)).await {
        Ok(response) => response,
        Err(_) => {
            tracing::warn!("Zeitlimit der Anfrage überschritten");
            ApiError::timeout().into_response()
        }
    }
}

/// The limit of a request: [`UPLOAD_TIMEOUT`] for the upload, `default` for all others.
fn limit_for(method: &Method, path: &str, default: Duration) -> Duration {
    if method == Method::POST && path == crate::documents::UPLOAD_PATH {
        UPLOAD_TIMEOUT
    } else {
        default
    }
}

#[cfg(test)]
mod tests {
    use super::{Duration, Method, UPLOAD_TIMEOUT, limit_for};

    #[test]
    fn only_the_upload_gets_the_long_limit() {
        let default = Duration::from_secs(30);
        assert_eq!(
            limit_for(&Method::POST, "/api/documents", default),
            UPLOAD_TIMEOUT
        );
        assert_eq!(limit_for(&Method::GET, "/api/documents", default), default);
        assert_eq!(
            limit_for(&Method::POST, "/api/documents/x", default),
            default
        );
        assert_eq!(
            limit_for(&Method::POST, "/api/properties", default),
            default
        );
    }
}
