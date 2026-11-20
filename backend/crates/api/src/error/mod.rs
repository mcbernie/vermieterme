//! The one error shape of the web API (`docs/api-contract.md`, "Fehlerform"):
//! `{"error": string, "code"?: string, "fields"?: {field: message}}`.
//!
//! Handlers return `Result<_, ApiError>`. Messages are German and generic; internal
//! details, personal data and secrets never reach the client and are not logged either
//! (database errors only with kind and SQLSTATE/code, decision 35).
//!
//! Submodules: [`json`] (request extractors with this error shape, timestamp format),
//! [`validate`] (trimming and length checks against the schema limits), [`db_log`]
//! (what of a database error may be logged), [`fallback`] (unknown paths and methods).

pub mod db_log;
pub mod fallback;
pub mod json;
pub mod validate;

use std::borrow::Cow;
use std::time::Duration;

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use vermieterme_db::DbError;

/// Machine-readable error codes of the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// 400: invalid input.
    Validation,
    /// 401: no or expired session.
    Unauthenticated,
    /// 403: role insufficient.
    Forbidden,
    /// 403: CSRF header missing or foreign origin.
    Csrf,
    /// 404: missing or foreign.
    NotFound,
    /// 405: the path exists, the method does not (not yet in the contract table).
    MethodNotAllowed,
    /// 409: uniqueness or state conflict.
    Conflict,
    /// 409: the sent version is outdated (`current` carries the stored state, #67).
    VersionConflict,
    /// 409: a billing period overlaps another one of the property.
    Overlap,
    /// 422: the shares of a unit do not allow the MEA share (#85).
    InvalidShares,
    /// 413: request body too large.
    TooLarge,
    /// 429: too many attempts.
    RateLimited,
    /// 502: an external service failed.
    Upstream,
    /// 500: internal error.
    Internal,
}

/// An error response of the web API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    status: StatusCode,
    code: ErrorCode,
    message: Cow<'static, str>,
    fields: Vec<(String, String)>,
    retry_after: Option<Duration>,
    current: Option<serde_json::Value>,
}

/// JSON body of an error.
#[derive(Serialize)]
struct Body<'a> {
    error: &'a str,
    code: ErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    fields: Option<std::collections::BTreeMap<&'a str, &'a str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current: Option<&'a serde_json::Value>,
}

impl ApiError {
    /// Builds an error without fields.
    fn new(status: StatusCode, code: ErrorCode, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            fields: Vec::new(),
            retry_after: None,
            current: None,
        }
    }

    /// 400 `validation` for one field; the message is also the top-level `error`.
    pub fn field(field: &str, message: impl Into<Cow<'static, str>>) -> Self {
        let message = message.into();
        Self::validation(vec![(field.to_owned(), message.into_owned())])
    }

    /// 400 `validation` with several fields; `error` is the first message. Callers pass at
    /// least one field (an empty list yields a generic message).
    pub fn validation(fields: Vec<(String, String)>) -> Self {
        let message = fields
            .first()
            .map_or_else(|| "Eingabe ungültig".to_owned(), |(_, m)| m.clone());
        Self {
            fields,
            ..Self::new(StatusCode::BAD_REQUEST, ErrorCode::Validation, message)
        }
    }

    /// 400 `validation` without a field (e.g. a malformed body).
    pub fn bad_request(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, ErrorCode::Validation, message)
    }

    /// 401 with a generic message (no session) or a specific one (login failed).
    pub fn unauthenticated(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            ErrorCode::Unauthenticated,
            message,
        )
    }

    /// 401 "Nicht angemeldet".
    pub fn no_session() -> Self {
        Self::unauthenticated("Nicht angemeldet")
    }

    /// 403 `forbidden`.
    pub fn forbidden(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(StatusCode::FORBIDDEN, ErrorCode::Forbidden, message)
    }

    /// 403 `csrf` (exact contract text).
    pub fn csrf() -> Self {
        Self::new(StatusCode::FORBIDDEN, ErrorCode::Csrf, "Anfrage abgelehnt")
    }

    /// 404 `not_found` with a specific message.
    pub fn not_found(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(StatusCode::NOT_FOUND, ErrorCode::NotFound, message)
    }

    /// 405 `method_not_allowed` (generic text).
    pub fn method_not_allowed() -> Self {
        Self::new(
            StatusCode::METHOD_NOT_ALLOWED,
            ErrorCode::MethodNotAllowed,
            "Methode nicht erlaubt",
        )
    }

    /// 413 `too_large` (generic text).
    pub fn too_large() -> Self {
        Self::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            ErrorCode::TooLarge,
            "Anfrage zu groß",
        )
    }

    /// 409 `conflict`.
    pub fn conflict(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(StatusCode::CONFLICT, ErrorCode::Conflict, message)
    }

    /// 409 `overlap`: the billing period overlaps another one.
    pub fn overlap(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(StatusCode::CONFLICT, ErrorCode::Overlap, message)
    }

    /// 409 `version_conflict` with the stored state in `current`, so the client can show
    /// or merge it (#67).
    pub fn version_conflict(current: serde_json::Value) -> Self {
        Self {
            current: Some(current),
            ..Self::new(
                StatusCode::CONFLICT,
                ErrorCode::VersionConflict,
                "Der Eintrag wurde inzwischen geändert. Bitte neu laden.",
            )
        }
    }

    /// 422 `invalid_shares`: a calculation needs shares the stored data cannot give.
    pub fn invalid_shares(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            ErrorCode::InvalidShares,
            message,
        )
    }

    /// 429 `rate_limited` with `Retry-After` (whole seconds, at least 1).
    pub fn rate_limited(retry_after: Duration) -> Self {
        Self {
            retry_after: Some(retry_after),
            ..Self::new(
                StatusCode::TOO_MANY_REQUESTS,
                ErrorCode::RateLimited,
                "Zu viele Versuche. Bitte später erneut versuchen.",
            )
        }
    }

    /// 502 `upstream`: an external service (Bundesbank) failed; `message` is generic.
    pub fn upstream(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(StatusCode::BAD_GATEWAY, ErrorCode::Upstream, message)
    }

    /// 500 `internal` (exact contract text, no details).
    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::Internal,
            "Interner Fehler",
        )
    }

    /// HTTP status.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Error code.
    pub fn code(&self) -> ErrorCode {
        self.code
    }

    /// Human-readable message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl IntoResponse for ApiError {
    /// Serialises the contract shape; adds `Retry-After` for 429.
    fn into_response(self) -> Response {
        let fields = (!self.fields.is_empty()).then(|| {
            self.fields
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect()
        });
        let body = Body {
            error: &self.message,
            code: self.code,
            fields,
            current: self.current.as_ref(),
        };
        let mut response = (self.status, axum::Json(body)).into_response();
        if let Some(wait) = self.retry_after {
            let secs = wait
                .as_secs()
                .saturating_add(u64::from(wait.subsec_nanos() > 0))
                .max(1);
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from(secs));
        }
        response
    }
}

impl From<DbError> for ApiError {
    /// Contract mapping: not found -> 404, conflict -> 409, constraint -> 400; anything
    /// else is logged (kind and code only) and becomes 500.
    fn from(error: DbError) -> Self {
        match error {
            DbError::NotFound => Self::not_found("Nicht gefunden"),
            DbError::Conflict => Self::conflict("Eintrag existiert bereits"),
            DbError::Constraint => Self::bad_request("Eingabe ungültig (z. B. zu lang)"),
            other => {
                db_log::log(&other, "Datenbankfehler");
                Self::internal()
            }
        }
    }
}

/// Maps a database error like `From`, but a missing (or foreign) row gets the specific
/// 404 `message`. Use as `.map_err(not_found_as("Kunde nicht gefunden"))`.
pub fn not_found_as(message: &'static str) -> impl FnOnce(DbError) -> ApiError {
    move |error| match error {
        DbError::NotFound => ApiError::not_found(message),
        other => ApiError::from(other),
    }
}

/// Result alias of handlers.
pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests;
