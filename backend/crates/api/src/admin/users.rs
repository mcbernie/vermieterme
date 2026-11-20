//! User management of the administration: users of an account, invitation into an
//! account, lock and role, reset links.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use vermieterme_db::repo::account;
use vermieterme_db::repo::user::{self, NewMember, Role, UserPatch};
use vermieterme_db::repo::user_token::TokenPurpose;
use vermieterme_db::{DbError, in_transaction};

use super::{ACCOUNT_NOT_FOUND, record};
use crate::auth::{AdminCtx, links};
use crate::error::json::{ApiJson, timestamp};
use crate::error::{ApiError, ApiResult, not_found_as};
use crate::routes::AppState;
use crate::users::{
    InvitationBody, InvitationJson, ResetLinkJson, USER_NOT_FOUND, UserJson, email_taken,
};

/// Members of an account.
pub async fn list(
    State(state): State<AppState>,
    ctx: AdminCtx,
    Path(id): Path<String>,
) -> ApiResult<axum::Json<Vec<UserJson>>> {
    let mut conn = state.conn().await?;
    let account = account::get(&mut conn, &ctx.scope, &id)
        .await
        .map_err(not_found_as(ACCOUNT_NOT_FOUND))?;
    let users = user::list(&mut conn, &ctx.scope.act_in(account.id)).await?;
    Ok(axum::Json(users.into_iter().map(UserJson::from).collect()))
}

/// Invites a member into an account (one-time link).
pub async fn invite(
    State(state): State<AppState>,
    ctx: AdminCtx,
    Path(id): Path<String>,
    ApiJson(body): ApiJson<InvitationBody>,
) -> ApiResult<Response> {
    let (name, email) = body.validate()?;
    let mut conn = state.conn().await?;
    let account = account::get(&mut conn, &ctx.scope, &id)
        .await
        .map_err(not_found_as(ACCOUNT_NOT_FOUND))?;
    let (created, link) = in_transaction(&mut conn, async |tx| {
        let new = NewMember {
            email,
            name,
            password_hash: None,
        };
        let created = user::create(tx, &ctx.scope.act_in(&account.id), &new).await?;
        let link = links::issue(tx, &created.id, TokenPurpose::Invite).await?;
        record(
            tx,
            &ctx,
            Some(&account.id),
            "user.invite",
            "user",
            &created.id,
        )
        .await?;
        Ok((created, link))
    })
    .await
    .map_err(email_taken)?;
    let body = InvitationJson {
        user: created.into(),
        invite_url: link.url,
        expires_at: timestamp(link.expires_at),
    };
    Ok((StatusCode::CREATED, axum::Json(body)).into_response())
}

/// Request of `PATCH /api/admin/users/{id}`.
#[derive(Debug, Deserialize)]
pub struct UpdateBody {
    locked: Option<bool>,
    role: Option<String>,
}

/// Locks/unlocks a user or changes the role. Not for the own user (400); the last
/// active admin cannot be locked or demoted (409).
pub async fn update(
    State(state): State<AppState>,
    ctx: AdminCtx,
    Path(id): Path<String>,
    ApiJson(body): ApiJson<UpdateBody>,
) -> ApiResult<axum::Json<UserJson>> {
    if id == ctx.user.id {
        return Err(ApiError::bad_request(
            "Das eigene Admin-Konto kann hier nicht geändert werden",
        ));
    }
    let role = match body.role.as_deref() {
        None => None,
        Some(code) => {
            Some(Role::from_code(code).ok_or_else(|| ApiError::field("role", "Unbekannte Rolle"))?)
        }
    };
    let patch = UserPatch {
        name: None,
        role,
        locked: body.locked,
    };
    let action = match (role, body.locked) {
        (Some(_), _) => "user.role",
        (None, Some(true)) => "user.lock",
        (None, Some(false)) => "user.unlock",
        (None, None) => "user.update",
    };
    let mut conn = state.conn().await?;
    let updated = in_transaction(&mut conn, async |tx| {
        let updated = user::update_for_admin(tx, &ctx.scope, &id, &patch).await?;
        record(
            tx,
            &ctx,
            updated.account_id.as_deref(),
            action,
            "user",
            &updated.id,
        )
        .await?;
        Ok(updated)
    })
    .await
    .map_err(|error| match error {
        DbError::NotFound => ApiError::not_found(USER_NOT_FOUND),
        DbError::Conflict => ApiError::conflict(
            "Der letzte aktive Admin kann nicht gesperrt oder herabgestuft werden",
        ),
        DbError::Constraint => {
            ApiError::field("role", "Ein Benutzer ohne Kunde kann kein Mitglied sein")
        }
        other => ApiError::from(other),
    })?;
    Ok(axum::Json(updated.into()))
}

/// Issues a password reset link for any user.
pub async fn reset_link(
    State(state): State<AppState>,
    ctx: AdminCtx,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let mut conn = state.conn().await?;
    let link = in_transaction(&mut conn, async |tx| {
        let target = user::get_for_admin(tx, &ctx.scope, &id).await?;
        let link = links::issue(tx, &target.id, TokenPurpose::Reset).await?;
        record(
            tx,
            &ctx,
            target.account_id.as_deref(),
            "user.reset_link",
            "user",
            &target.id,
        )
        .await?;
        Ok(link)
    })
    .await
    .map_err(not_found_as(USER_NOT_FOUND))?;
    Ok((StatusCode::CREATED, axum::Json(ResetLinkJson::from(link))).into_response())
}
