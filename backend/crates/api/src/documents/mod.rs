//! Documents of an account (contract section 8, P6): list, upload, download, delete.
//!
//! Files live below `UPLOAD_DIR/{account_id}/` under a generated name ([`storage`]); the
//! uploaded name is only shown, never used as a path. Types are recognised by content
//! ([`file_type`]). Deleting a document, and every cascading delete that takes documents
//! along ([`cascade`]), removes the files after the commit (#106). Changes run with their
//! audit entry in one transaction, like master data.

pub(crate) mod cascade;
pub mod download;
pub mod file_type;
pub mod storage;
pub mod upload;

use axum::extract::{DefaultBodyLimit, Path, State};
use axum::response::Response;
use axum::routing::get;
use serde::{Deserialize, Serialize};
use vermieterme_db::repo::billing::period;
use vermieterme_db::repo::documents::{self, Document};
use vermieterme_db::repo::masterdata::tenant;

use crate::auth::AccountCtx;
use crate::error::json::{ApiQuery, timestamp};
use crate::error::{ApiError, ApiResult, not_found_as};
use crate::masterdata::deleted;
use crate::masterdata::fields::filter;
use crate::routes::{AppRouter, AppState};

/// Largest accepted file (contract: 10 MB).
pub const MAX_FILE_BYTES: usize = 10 * 1024 * 1024;
/// Body limit of the upload route: the file plus room for the other form fields and the
/// multipart framing (all other routes keep axum's 2 MB).
const UPLOAD_BODY_MAX: usize = MAX_FILE_BYTES + 64 * 1024;

/// Message for a missing or foreign document.
pub const NOT_FOUND: &str = "Dokument nicht gefunden";
/// Message for a missing or foreign tenant in a filter or assignment.
const TENANT_NOT_FOUND: &str = "Mieter nicht gefunden";

/// Routes of the area.
pub fn router() -> AppRouter {
    AppRouter::new()
        .route(
            "/api/documents",
            get(list)
                .post(upload::create)
                .layer(DefaultBodyLimit::max(UPLOAD_BODY_MAX)),
        )
        .route("/api/documents/{id}", axum::routing::delete(remove))
        .route("/api/documents/{id}/file", get(download::file))
}

/// A document in responses (`Document` of the contract).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentJson {
    id: String,
    original_name: String,
    mime_type: String,
    size: i64,
    category: String,
    billing_period_id: Option<String>,
    tenant_id: Option<String>,
    created_at: String,
}

impl From<Document> for DocumentJson {
    /// Response form with ISO timestamp; the stored file name is never sent.
    fn from(d: Document) -> Self {
        Self {
            id: d.id,
            original_name: d.original_name,
            mime_type: d.mime_type,
            size: d.size_bytes,
            category: d.category,
            billing_period_id: d.billing_period_id,
            tenant_id: d.tenant_id,
            created_at: timestamp(d.created_at),
        }
    }
}

/// Query of the list.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentQuery {
    billing_period_id: Option<String>,
    tenant_id: Option<String>,
}

/// Checks that a billing period and a tenant (each optional) exist in the account, so
/// the 404 names the right object (the repository re-checks inside its transaction).
pub(crate) async fn check_assignment(
    state: &AppState,
    ctx: &AccountCtx,
    billing_period_id: Option<&str>,
    tenant_id: Option<&str>,
) -> ApiResult<()> {
    let mut conn = state.conn().await?;
    if let Some(id) = billing_period_id {
        period::get(&mut conn, &ctx.scope, id)
            .await
            .map_err(not_found_as(crate::billing::NOT_FOUND))?;
    }
    if let Some(id) = tenant_id {
        tenant::get(&mut conn, &ctx.scope, id)
            .await
            .map_err(not_found_as(TENANT_NOT_FOUND))?;
    }
    Ok(())
}

/// Documents of the account, newest first, optionally of a billing period and/or a
/// tenant (an unknown or foreign one is 404).
pub async fn list(
    State(state): State<AppState>,
    ctx: AccountCtx,
    ApiQuery(query): ApiQuery<DocumentQuery>,
) -> ApiResult<axum::Json<Vec<DocumentJson>>> {
    let billing_period_id = filter(query.billing_period_id.as_deref());
    let tenant_id = filter(query.tenant_id.as_deref());
    check_assignment(&state, &ctx, billing_period_id, tenant_id).await?;
    let mut conn = state.conn().await?;
    let found = documents::list(&mut conn, &ctx.scope, billing_period_id, tenant_id).await?;
    Ok(axum::Json(
        found.into_iter().map(DocumentJson::from).collect(),
    ))
}

/// Deletes a document and, after the commit, its file.
pub async fn remove(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    cascade::delete_with_files(
        &state,
        &ctx,
        ("document.delete", "document"),
        NOT_FOUND,
        &id,
        async |tx| Ok(vec![documents::delete(tx, &ctx.scope, &id).await?]),
    )
    .await?;
    Ok(deleted())
}

/// A row of the change moved to another property while it ran (rare race between an
/// upload or delete and a move of the unit or tenant): 409 with a request to retry
/// instead of the generic uniqueness text.
pub(crate) fn changed_meanwhile(error: ApiError) -> ApiError {
    if error.status() == axum::http::StatusCode::CONFLICT {
        ApiError::conflict("Die Zuordnung wurde gleichzeitig geändert. Bitte erneut versuchen.")
    } else {
        error
    }
}
