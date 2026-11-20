//! `GET /api/profile` and `PUT /api/profile`: the signed-in user's name and password.

use axum::extract::State;
use serde::{Deserialize, Serialize};
use vermieterme_db::in_transaction;
use vermieterme_db::repo::session;
use vermieterme_db::repo::user::{self, User};

use crate::auth::password::Verification;
use crate::auth::rate_limit::PASSWORD_CHANGE;
use crate::auth::{CurrentUser, links};
use crate::error::json::ApiJson;
use crate::error::validate::{NAME_MAX, Validator};
use crate::error::{ApiError, ApiResult};
use crate::routes::AppState;

/// Profile response.
#[derive(Debug, Serialize)]
pub struct ProfileJson {
    id: String,
    name: String,
    email: String,
}

impl From<User> for ProfileJson {
    /// Public fields of the own user.
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
        }
    }
}

/// Request of `PUT /api/profile`; absent fields stay unchanged.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileBody {
    name: Option<String>,
    current_password: Option<String>,
    new_password: Option<String>,
}

/// The own profile.
pub async fn get_profile(
    State(state): State<AppState>,
    current: CurrentUser,
) -> ApiResult<axum::Json<ProfileJson>> {
    let mut conn = state.conn().await?;
    let me = user::get_self(&mut conn, &current.scope()).await?;
    Ok(axum::Json(me.into()))
}

/// Changes name and/or password. A password change needs the current password (checked
/// against the stored hash only, never against `ADMIN_PASSWORD`, #69) and ends all other
/// sessions of the user and all open reset links.
pub async fn update_profile(
    State(state): State<AppState>,
    current: CurrentUser,
    ApiJson(body): ApiJson<ProfileBody>,
) -> ApiResult<axum::Json<ProfileJson>> {
    let mut v = Validator::new();
    let name = body
        .name
        .as_deref()
        .map(|n| v.text("name", "Name", n, NAME_MAX));
    if let Some(new) = &body.new_password {
        v.password("newPassword", new);
        if body.current_password.is_none() {
            v.fail("currentPassword", "Aktuelles Passwort fehlt");
        }
    }
    v.finish()?;
    let new_hash = match (body.new_password, body.current_password) {
        (Some(new), Some(old)) => Some(check_and_hash(&state, &current, old, new).await?),
        _ => None,
    };
    let scope = current.scope();
    let mut conn = state.conn().await?;
    let me = in_transaction(&mut conn, async |tx| {
        if let Some(name) = &name {
            user::set_own_name(tx, &scope, name).await?;
        }
        if let Some(hash) = &new_hash {
            user::set_password_hash(tx, &scope, hash).await?;
            session::delete_for_user(tx, scope.user_id(), Some(&current.session_id)).await?;
            links::revoke(tx, scope.user_id()).await?;
        }
        user::get_self(tx, &scope).await
    })
    .await?;
    Ok(axum::Json(me.into()))
}

/// Verifies the current password (rate-limited per user) and hashes the new one.
async fn check_and_hash(
    state: &AppState,
    current: &CurrentUser,
    old: String,
    new: String,
) -> ApiResult<vermieterme_db::PasswordHash> {
    let key = current.user.id.as_str();
    let attempt = state
        .limiter
        .begin(&PASSWORD_CHANGE, key)
        .map_err(ApiError::rate_limited)?;
    let mut conn = state.conn().await?;
    let stored = user::find_login(&mut conn, &current.user.email)
        .await?
        .filter(|r| r.user.id == current.user.id)
        .and_then(|r| r.password_hash);
    drop(conn);
    if state.hasher.verify(old, stored).await? == Verification::Invalid {
        attempt.failed();
        return Err(ApiError::field(
            "currentPassword",
            "Aktuelles Passwort ist falsch",
        ));
    }
    attempt.succeeded();
    state.hasher.hash(new).await
}
