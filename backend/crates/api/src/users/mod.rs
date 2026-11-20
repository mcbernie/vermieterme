//! Users of an account and the own profile (contract section 5).
//!
//! - `GET /api/profile`, `PUT /api/profile` – session ([`profile`])
//! - `GET /api/users` – members of the own account
//! - `POST /api/users/invitations` – invite a member (one-time link)
//! - `DELETE /api/users/{id}` – delete a member (not oneself)
//! - `POST /api/users/{id}/reset-link` – password reset link for a member

mod profile;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use serde::{Deserialize, Serialize};
use vermieterme_db::repo::user::{self, NewMember, User};
use vermieterme_db::repo::user_token::TokenPurpose;
use vermieterme_db::{DbError, in_transaction};

use crate::auth::AccountCtx;
use crate::auth::links;
use crate::error::json::{ApiJson, timestamp};
use crate::error::validate::{NAME_MAX, Validator};
use crate::error::{ApiError, ApiResult, not_found_as};
use crate::routes::{AppRouter, AppState};

/// Message for a missing or foreign user.
pub const USER_NOT_FOUND: &str = "Benutzer nicht gefunden";
/// Message for a taken e-mail address (contract).
pub const EMAIL_TAKEN: &str = "E-Mail existiert bereits";

/// Routes of the area.
pub fn router() -> AppRouter {
    AppRouter::new()
        .route(
            "/api/profile",
            get(profile::get_profile).put(profile::update_profile),
        )
        .route("/api/users", get(list))
        .route("/api/users/invitations", post(invite))
        .route("/api/users/{id}", delete(remove))
        .route("/api/users/{id}/reset-link", post(reset_link))
}

/// A user in API responses (no credentials).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UserJson {
    /// Id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// E-mail address.
    pub email: String,
    /// `admin` or `member`.
    pub role: &'static str,
    /// Locked users cannot sign in.
    pub locked: bool,
}

impl From<User> for UserJson {
    /// Drops everything but the public fields.
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
            role: user.role.code(),
            locked: user.locked,
        }
    }
}

/// Response of an invitation: the new user and the one-time link (shown once).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitationJson {
    /// The invited user (no password yet).
    pub user: UserJson,
    /// Relative link, token in the fragment.
    pub invite_url: String,
    /// End of validity.
    pub expires_at: String,
}

/// Response of a reset link (shown once).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetLinkJson {
    /// Relative link, token in the fragment.
    pub reset_url: String,
    /// End of validity.
    pub expires_at: String,
}

impl From<links::IssuedLink> for ResetLinkJson {
    /// Formats the link for the response.
    fn from(link: links::IssuedLink) -> Self {
        Self {
            reset_url: link.url,
            expires_at: timestamp(link.expires_at),
        }
    }
}

/// Request of an invitation.
#[derive(Debug, Deserialize)]
pub struct InvitationBody {
    name: String,
    email: String,
}

impl InvitationBody {
    /// Validated, trimmed values (name, e-mail).
    pub fn validate(&self) -> ApiResult<(String, String)> {
        let mut v = Validator::new();
        let name = v.text("name", "Name", &self.name, NAME_MAX);
        let email = v.email("email", &self.email);
        v.finish()?;
        Ok((name, email))
    }
}

/// Maps a conflict on insert to the contract message for taken addresses.
pub fn email_taken(error: DbError) -> ApiError {
    match error {
        DbError::Conflict => ApiError::conflict(EMAIL_TAKEN),
        other => ApiError::from(other),
    }
}

/// Members of the own account.
async fn list(
    State(state): State<AppState>,
    ctx: AccountCtx,
) -> ApiResult<axum::Json<Vec<UserJson>>> {
    let mut conn = state.conn().await?;
    let users = user::list(&mut conn, &ctx.scope).await?;
    Ok(axum::Json(users.into_iter().map(UserJson::from).collect()))
}

/// Invites a member into the own account: creates the user without password and a
/// one-time invitation link.
async fn invite(
    State(state): State<AppState>,
    ctx: AccountCtx,
    ApiJson(body): ApiJson<InvitationBody>,
) -> ApiResult<Response> {
    let (name, email) = body.validate()?;
    let mut conn = state.conn().await?;
    let (created, link) = in_transaction(&mut conn, async |tx| {
        let new = NewMember {
            email,
            name,
            password_hash: None,
        };
        let created = user::create(tx, &ctx.scope, &new).await?;
        let link = links::issue(tx, &created.id, TokenPurpose::Invite).await?;
        ctx.audit(tx, "user.invite", "user", Some(&created.id))
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

/// Deletes a member of the own account; oneself -> 400, foreign -> 404.
async fn remove(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if id == ctx.user.id {
        return Err(ApiError::bad_request(
            "Sie können sich nicht selbst löschen",
        ));
    }
    let mut conn = state.conn().await?;
    in_transaction(&mut conn, async |tx| {
        user::delete(tx, &ctx.scope, &id).await?;
        ctx.audit(tx, "user.delete", "user", Some(&id)).await
    })
    .await
    .map_err(not_found_as(USER_NOT_FOUND))?;
    Ok(StatusCode::NO_CONTENT)
}

/// Issues a password reset link for a member of the own account.
async fn reset_link(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let mut conn = state.conn().await?;
    let link = in_transaction(&mut conn, async |tx| {
        let member = user::get(tx, &ctx.scope, &id).await?;
        let link = links::issue(tx, &member.id, TokenPurpose::Reset).await?;
        ctx.audit(tx, "user.reset_link", "user", Some(&member.id))
            .await?;
        Ok(link)
    })
    .await
    .map_err(not_found_as(USER_NOT_FOUND))?;
    Ok((StatusCode::CREATED, axum::Json(ResetLinkJson::from(link))).into_response())
}
