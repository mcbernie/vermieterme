//! `GET/POST /api/admin/accounts`, `PATCH /api/admin/accounts/{id}`.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use vermieterme_db::in_transaction;
use vermieterme_db::repo::account::{self, Account, AccountPatch};

use super::{ACCOUNT_NOT_FOUND, record};
use crate::auth::AdminCtx;
use crate::error::json::{ApiJson, timestamp};
use crate::error::validate::{NAME_MAX, Validator};
use crate::error::{ApiResult, not_found_as};
use crate::routes::AppState;

/// An account in responses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountJson {
    id: String,
    name: String,
    locked: bool,
    created_at: String,
    /// Only in the list.
    #[serde(skip_serializing_if = "Option::is_none")]
    user_count: Option<i64>,
}

impl AccountJson {
    /// Response form of `account`, with the user count in the list.
    fn new(account: Account, user_count: Option<i64>) -> Self {
        Self {
            id: account.id,
            name: account.name,
            locked: account.locked,
            created_at: timestamp(account.created_at),
            user_count,
        }
    }
}

/// All accounts with their number of users.
pub async fn list(
    State(state): State<AppState>,
    ctx: AdminCtx,
) -> ApiResult<axum::Json<Vec<AccountJson>>> {
    let mut conn = state.conn().await?;
    let accounts = account::list(&mut conn, &ctx.scope).await?;
    let items = accounts
        .into_iter()
        .map(|s| AccountJson::new(s.account, Some(s.user_count)))
        .collect();
    Ok(axum::Json(items))
}

/// Request of `POST /api/admin/accounts`.
#[derive(Debug, Deserialize)]
pub struct CreateBody {
    name: String,
}

/// Creates an account.
pub async fn create(
    State(state): State<AppState>,
    ctx: AdminCtx,
    ApiJson(body): ApiJson<CreateBody>,
) -> ApiResult<Response> {
    let mut v = Validator::new();
    let name = v.text("name", "Name", &body.name, NAME_MAX);
    v.finish()?;
    let mut conn = state.conn().await?;
    let created = in_transaction(&mut conn, async |tx| {
        let created = account::create(tx, &ctx.scope, &name).await?;
        record(
            tx,
            &ctx,
            Some(&created.id),
            "account.create",
            "account",
            &created.id,
        )
        .await?;
        Ok(created)
    })
    .await?;
    Ok((
        StatusCode::CREATED,
        axum::Json(AccountJson::new(created, None)),
    )
        .into_response())
}

/// Request of `PATCH /api/admin/accounts/{id}`; absent fields stay unchanged.
#[derive(Debug, Deserialize)]
pub struct UpdateBody {
    name: Option<String>,
    locked: Option<bool>,
}

/// Renames and/or locks an account. A lock ends the sessions of its members (in the
/// repository, same transaction); tenant tokens stop working because every tenant
/// request checks the account lock (P7).
pub async fn update(
    State(state): State<AppState>,
    ctx: AdminCtx,
    Path(id): Path<String>,
    ApiJson(body): ApiJson<UpdateBody>,
) -> ApiResult<axum::Json<AccountJson>> {
    let mut v = Validator::new();
    let name = body
        .name
        .as_deref()
        .map(|n| v.text("name", "Name", n, NAME_MAX));
    v.finish()?;
    let patch = AccountPatch {
        name,
        locked: body.locked,
    };
    let action = match body.locked {
        Some(true) => "account.lock",
        Some(false) => "account.unlock",
        None => "account.update",
    };
    let mut conn = state.conn().await?;
    let updated = in_transaction(&mut conn, async |tx| {
        let updated = account::update(tx, &ctx.scope, &id, &patch).await?;
        record(tx, &ctx, Some(&updated.id), action, "account", &updated.id).await?;
        Ok(updated)
    })
    .await
    .map_err(not_found_as(ACCOUNT_NOT_FOUND))?;
    Ok(axum::Json(AccountJson::new(updated, None)))
}
