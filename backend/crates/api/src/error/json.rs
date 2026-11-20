//! JSON in and out with the contract's error shape: body and query extractors whose
//! rejections are [`ApiError`]s (axum's own rejections are plain text), and the
//! timestamp format of the web API.

use axum::extract::{FromRequest, FromRequestParts, Query, Request};
use axum::http::StatusCode;
use axum::http::request::Parts;
use serde::de::DeserializeOwned;
use time::OffsetDateTime;
use time::macros::format_description;

use super::ApiError;

/// JSON request body; a malformed body or wrong content type -> 400 `validation`, a
/// body over the size limit -> 413 `too_large`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ApiJson<T>(pub T);

impl<T, S> FromRequest<S> for ApiJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    /// Delegates to [`axum::Json`] and replaces its rejection. The rejection text is not
    /// passed on: it may quote parts of the input.
    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        axum::Json::<T>::from_request(req, state)
            .await
            .map(|axum::Json(value)| Self(value))
            .map_err(|rejection| {
                if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
                    ApiError::too_large()
                } else {
                    ApiError::bad_request("Ungültige Anfrage (JSON erwartet)")
                }
            })
    }
}

/// Query string; malformed -> 400 `validation`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ApiQuery<T>(pub T);

impl<T, S> FromRequestParts<S> for ApiQuery<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    /// Delegates to [`axum::extract::Query`] and replaces its rejection.
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Query::<T>::from_request_parts(parts, state)
            .await
            .map(|Query(value)| Self(value))
            .map_err(|_| ApiError::bad_request("Ungültige Abfrageparameter"))
    }
}

/// Formats a stored timestamp (Unix ms UTC) as the contract's
/// `"YYYY-MM-DDTHH:MM:SS.mmmZ"`. Out-of-range values (never stored) fall back to the epoch.
pub fn timestamp(millis: i64) -> String {
    let format =
        format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z");
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(millis) * 1_000_000)
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
        .format(format)
        .unwrap_or_else(|_| "1970-01-01T00:00:00.000Z".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{ApiJson, timestamp};
    use axum::body::Body;
    use axum::extract::{FromRequest, Request};
    use axum::http::{StatusCode, header};

    /// Rejection of `body` posted as JSON.
    async fn reject(body: Vec<u8>) -> super::ApiError {
        let req = Request::builder()
            .method("POST")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();
        ApiJson::<serde_json::Value>::from_request(req, &())
            .await
            .unwrap_err()
    }

    #[tokio::test]
    async fn oversized_body_is_413_and_malformed_is_400() {
        let big = reject(vec![b' '; 3 * 1024 * 1024]).await;
        assert_eq!(big.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(big.code(), crate::error::ErrorCode::TooLarge);
        let bad = reject(b"{".to_vec()).await;
        assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn timestamps_have_milliseconds_and_z() {
        assert_eq!(timestamp(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(timestamp(1_769_860_800_123), "2026-01-31T12:00:00.123Z");
    }
}
