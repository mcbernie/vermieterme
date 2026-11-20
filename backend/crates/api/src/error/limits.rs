//! Errors of the resource limits: storage quota (decision 44) and request time limit
//! (decision 45).

use axum::http::StatusCode;

use super::{ApiError, ErrorCode};

impl ApiError {
    /// 413 `quota_exceeded`: the document would exceed the account's storage.
    pub fn quota_exceeded() -> Self {
        Self::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            ErrorCode::QuotaExceeded,
            "Speicherplatz für Dokumente ist aufgebraucht. Bitte löschen Sie nicht mehr benötigte Dokumente.",
        )
    }

    /// 503 `timeout`: the request took longer than its time limit. 503 rather than 408:
    /// the server gives up, not necessarily the client was slow, and clients may resend a
    /// request answered with 408 on their own (also a `POST`).
    pub fn timeout() -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::Timeout,
            "Zeitüberschreitung der Anfrage. Bitte erneut versuchen.",
        )
    }
}
