//! `POST /api/admin/switch` (work inside a customer) and `DELETE /api/admin/switch`
//! (back to the own account). Both are audited; the session document then reports
//! `impersonating`, so the interface shows its notice bar.

use axum::extract::State;
use serde::Deserialize;
use vermieterme_db::in_transaction;
use vermieterme_db::repo::session;

use super::{ACCOUNT_NOT_FOUND, record};
use crate::auth::AdminCtx;
use crate::auth::session::{SessionJson, describe};
use crate::error::json::ApiJson;
use crate::error::{ApiError, ApiResult, not_found_as};
use crate::routes::AppState;

/// Request of `POST /api/admin/switch`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchBody {
    account_id: String,
}

/// Switches the admin's session into an account.
pub async fn enter(
    State(state): State<AppState>,
    ctx: AdminCtx,
    ApiJson(body): ApiJson<SwitchBody>,
) -> ApiResult<axum::Json<SessionJson>> {
    let target = body.account_id.trim().to_owned();
    if target.is_empty() {
        return Err(ApiError::field("accountId", "Kunde fehlt"));
    }
    let mut conn = state.conn().await?;
    in_transaction(&mut conn, async |tx| {
        session::set_acting_account(tx, &ctx.scope, &ctx.session_id, Some(&target)).await?;
        record(
            tx,
            &ctx,
            Some(&target),
            "account.switch",
            "account",
            &target,
        )
        .await
    })
    .await
    .map_err(not_found_as(ACCOUNT_NOT_FOUND))?;
    Ok(axum::Json(
        describe(&mut conn, &ctx.user, Some(&target)).await?,
    ))
}

/// Returns the admin's session to the own account (no-op without a switch).
pub async fn leave(
    State(state): State<AppState>,
    ctx: AdminCtx,
) -> ApiResult<axum::Json<SessionJson>> {
    let mut conn = state.conn().await?;
    if let Some(previous) = ctx.acting_account_id.as_deref() {
        in_transaction(&mut conn, async |tx| {
            session::set_acting_account(tx, &ctx.scope, &ctx.session_id, None).await?;
            record(
                tx,
                &ctx,
                Some(previous),
                "account.switch_back",
                "account",
                previous,
            )
            .await
        })
        .await?;
    }
    Ok(axum::Json(describe(&mut conn, &ctx.user, None).await?))
}
