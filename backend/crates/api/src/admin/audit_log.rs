//! `GET /api/admin/audit-log?accountId=&cursor=`: newest first, at most 100 per page.

use axum::extract::State;
use serde::{Deserialize, Serialize};
use vermieterme_db::repo::audit::{self, AuditEntry, AuditFilter};

use super::cursor;
use crate::auth::AdminCtx;
use crate::error::json::{ApiQuery, timestamp};
use crate::error::{ApiError, ApiResult};
use crate::routes::AppState;

/// Query parameters.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditQuery {
    account_id: Option<String>,
    cursor: Option<String>,
}

/// One entry in the response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditItemJson {
    id: String,
    at: String,
    user_id: Option<String>,
    user_email: Option<String>,
    account_id: Option<String>,
    action: String,
    entity: String,
    entity_id: Option<String>,
}

impl From<AuditEntry> for AuditItemJson {
    /// Response form of an entry.
    fn from(e: AuditEntry) -> Self {
        Self {
            id: e.id,
            at: timestamp(e.at),
            user_id: e.user_id,
            user_email: e.user_email,
            account_id: e.account_id,
            action: e.action,
            entity: e.entity,
            entity_id: e.entity_id,
        }
    }
}

/// One page.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditPageJson {
    items: Vec<AuditItemJson>,
    next_cursor: Option<String>,
}

/// Lists audit entries, optionally of one account; an invalid cursor -> 400.
pub async fn list(
    State(state): State<AppState>,
    ctx: AdminCtx,
    ApiQuery(query): ApiQuery<AuditQuery>,
) -> ApiResult<axum::Json<AuditPageJson>> {
    let before = match query.cursor.as_deref().filter(|c| !c.is_empty()) {
        None => None,
        Some(text) => Some(
            cursor::decode(text).ok_or_else(|| ApiError::field("cursor", "Ungültiger Cursor"))?,
        ),
    };
    let filter = AuditFilter {
        account_id: query.account_id.filter(|a| !a.trim().is_empty()),
        before,
    };
    let mut conn = state.conn().await?;
    let page = audit::list(&mut conn, &ctx.scope, &filter).await?;
    Ok(axum::Json(AuditPageJson {
        items: page.items.into_iter().map(AuditItemJson::from).collect(),
        next_cursor: page.next.as_ref().map(cursor::encode),
    }))
}
