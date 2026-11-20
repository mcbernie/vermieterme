//! Serving a stored file: `GET /api/documents/{id}/file` here, and the same response for
//! the tenant app (P7) through [`file_response`].
//!
//! `Content-Type` comes from the database (only the allowed types are shown inline,
//! anything else from legacy rows is offered as a download), `X-Content-Type-Options:
//! nosniff` stops the browser from guessing, and `Content-Disposition` carries the name
//! per RFC 6266 with an ASCII fallback and the UTF-8 form of RFC 5987.

use std::io::ErrorKind;

use axum::extract::{Path, State};
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use vermieterme_db::repo::documents::{self, Document};

use super::NOT_FOUND;
use super::file_type::FileType;
use super::storage::{file_path, io_failure};
use crate::auth::AccountCtx;
use crate::error::{ApiError, ApiResult, not_found_as};
use crate::routes::AppState;

/// Message when the row exists but its file does not (contract: 404, not 500).
pub const FILE_MISSING: &str = "Datei nicht gefunden";

/// Sends the file of a document of the account.
pub async fn file(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let mut conn = state.conn().await?;
    let document = documents::get(&mut conn, &ctx.scope, &id)
        .await
        .map_err(not_found_as(NOT_FOUND))?;
    drop(conn);
    file_response(&state, ctx.scope.account_id(), &document).await
}

/// The response with the file of `document` (already checked to be visible to the
/// caller) from the directory of `account_id`. A missing file or an unsafe stored name
/// is 404 with [`FILE_MISSING`].
pub async fn file_response(
    state: &AppState,
    account_id: &str,
    document: &Document,
) -> ApiResult<Response> {
    let missing = || ApiError::not_found(FILE_MISSING);
    let path =
        file_path(&state.config.upload_dir, account_id, &document.file_name).ok_or_else(missing)?;
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Err(missing()),
        Err(error) => return Err(io_failure(&error, "Datei eines Dokuments nicht lesbar")),
    };
    let (content_type, disposition) = match FileType::from_mime(&document.mime_type) {
        Some(t) => (t.mime(), "inline"),
        None => ("application/octet-stream", "attachment"),
    };
    let disposition = content_disposition(disposition, &document.original_name);
    let headers = [
        (header::CONTENT_TYPE, HeaderValue::from_static(content_type)),
        (
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ),
        (
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&disposition).map_err(|_| ApiError::internal())?,
        ),
        (
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, no-store"),
        ),
    ];
    Ok((headers, bytes).into_response())
}

/// `inline` or `attachment` with `filename="…"` (ASCII only: other characters, quotes
/// and backslashes become `_`) and `filename*=UTF-8''…` (percent-encoded), RFC 6266
/// section 4.3 / RFC 5987.
pub fn content_disposition(kind: &str, name: &str) -> String {
    let fallback: String = name
        .chars()
        .map(|c| match c {
            ' '..='~' if c != '"' && c != '\\' => c,
            _ => '_',
        })
        .collect();
    let mut encoded = String::with_capacity(name.len() * 3);
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() || b"!#$&+-.^_`|~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    format!("{kind}; filename=\"{fallback}\"; filename*=UTF-8''{encoded}")
}

#[cfg(test)]
mod tests {
    use super::content_disposition;

    #[test]
    fn ascii_names_pass_unchanged() {
        assert_eq!(
            content_disposition("inline", "Rechnung-2026.pdf"),
            "inline; filename=\"Rechnung-2026.pdf\"; filename*=UTF-8''Rechnung-2026.pdf"
        );
    }

    #[test]
    fn umlauts_quotes_and_controls_are_encoded() {
        assert_eq!(
            content_disposition("inline", "Müll \"neu\".pdf"),
            "inline; filename=\"M_ll _neu_.pdf\"; filename*=UTF-8''M%C3%BCll%20%22neu%22.pdf"
        );
        let header = content_disposition("attachment", "a\r\nSet-Cookie: x;b\\c");
        assert!(!header.contains('\r') && !header.contains('\n'));
        assert!(header.starts_with("attachment; filename=\"a__Set-Cookie: x;b_c\""));
        assert!(header.ends_with("a%0D%0ASet-Cookie%3A%20x%3Bb%5Cc"));
    }
}
