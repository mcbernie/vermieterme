//! One-time links for web users (decision 22): issuing invitation and reset links, and
//! redeeming them (`POST /api/auth/accept-invite`, `POST /api/auth/reset-password`).
//!
//! The token travels in the URL **fragment** (`/accept-invite#token=…`): browsers never
//! send the fragment to a server, so the secret does not end up in access logs, proxies
//! or `Referer` headers (security baseline: no secrets in URLs or query strings). The
//! page reads it in the browser and posts it in the JSON body.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use vermieterme_db::repo::session as session_repo;
use vermieterme_db::repo::user;
use vermieterme_db::repo::user_token::{self, TokenPurpose};
use vermieterme_db::row::now_millis;
use vermieterme_db::{AnyConnection, DbResult, SecretToken, UserScope, in_transaction};

use super::client_ip::ClientIp;
use super::extract::millis;
use super::password;
use super::rate_limit::{LINK_IP, ip_key};
use super::session;
use super::timing::Deadline;
use crate::config::{PASSWORD_RESET_TTL, USER_INVITE_TTL};
use crate::error::json::ApiJson;
use crate::error::validate::{NAME_MAX, Validator};
use crate::error::{ApiError, ApiResult};
use crate::routes::AppState;

/// Answer to an unknown, used or expired link (no distinction to the outside).
const LINK_INVALID: &str = "Der Link ist ungültig oder abgelaufen";

/// An issued link: relative URL with the token in the fragment, and its end of validity.
#[derive(Debug, Clone)]
pub struct IssuedLink {
    /// Relative URL of the web page that redeems the link.
    pub url: String,
    /// End of validity (Unix ms).
    pub expires_at: i64,
}

/// Issues a new link for `user_id` (older unused links of the purpose stop working).
/// Callers have checked that the user is within their scope.
pub async fn issue(
    conn: &mut AnyConnection,
    user_id: &str,
    purpose: TokenPurpose,
) -> DbResult<IssuedLink> {
    let (page, ttl) = match purpose {
        TokenPurpose::Invite => ("/accept-invite", USER_INVITE_TTL),
        TokenPurpose::Reset => ("/reset-password", PASSWORD_RESET_TTL),
    };
    let token = SecretToken::generate();
    let now = now_millis();
    let expires_at = now.saturating_add(millis(ttl));
    user_token::issue(conn, user_id, purpose, &token, now, expires_at).await?;
    Ok(IssuedLink {
        url: format!("{page}#token={}", token.expose()),
        expires_at,
    })
}

/// Request of `POST /api/auth/accept-invite`.
#[derive(Debug, Deserialize)]
pub struct AcceptInviteBody {
    token: String,
    name: String,
    password: String,
}

/// Accepts an invitation: sets name and first password, signs in (public).
pub async fn accept_invite(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    ApiJson(body): ApiJson<AcceptInviteBody>,
) -> ApiResult<Response> {
    let deadline = Deadline::start();
    let result = guarded(&state, &ip_key(ip), async {
        let mut v = Validator::new();
        let name = v.text("name", "Name", &body.name, NAME_MAX);
        v.password("password", &body.password);
        v.finish()?;
        let hash = password::hash(body.password).await?;
        let token = SecretToken::from_client(body.token.trim());
        let mut conn = state.conn().await?;
        let user_id = in_transaction(&mut conn, async |tx| {
            let Some(user_id) = consume(tx, TokenPurpose::Invite, &token).await? else {
                return Ok(None);
            };
            let scope = link_scope(&user_id);
            user::set_own_name(tx, &scope, &name).await?;
            user::set_password_hash(tx, &scope, &hash).await?;
            Ok(Some(user_id))
        })
        .await?
        .ok_or_else(|| ApiError::unauthenticated(LINK_INVALID))?;
        let started = session::start(&mut conn, &state.config, &headers, &user_id)
            .await?
            .ok_or_else(|| ApiError::unauthenticated(LINK_INVALID))?;
        let document = session::describe(&mut conn, &started.active.user, None).await?;
        Ok(session::signed_in(&started, document, &state.config))
    })
    .await;
    deadline.finish(result).await
}

/// Request of `POST /api/auth/reset-password`.
#[derive(Debug, Deserialize)]
pub struct ResetBody {
    token: String,
    password: String,
}

/// Sets a new password through a reset link and ends all sessions of the user (public).
pub async fn reset_password(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    ApiJson(body): ApiJson<ResetBody>,
) -> ApiResult<Response> {
    let deadline = Deadline::start();
    let result = guarded(&state, &ip_key(ip), async {
        let mut v = Validator::new();
        v.password("password", &body.password);
        v.finish()?;
        let hash = password::hash(body.password).await?;
        let token = SecretToken::from_client(body.token.trim());
        let mut conn = state.conn().await?;
        in_transaction(&mut conn, async |tx| {
            let Some(user_id) = consume(tx, TokenPurpose::Reset, &token).await? else {
                return Ok(false);
            };
            user::set_password_hash(tx, &link_scope(&user_id), &hash).await?;
            session_repo::delete_for_user(tx, &user_id, None).await?;
            Ok(true)
        })
        .await?
        .then_some(StatusCode::NO_CONTENT.into_response())
        .ok_or_else(|| ApiError::unauthenticated(LINK_INVALID))
    })
    .await;
    deadline.finish(result).await
}

/// Runs a link redemption under the per-address limit; an invalid link counts as failure.
async fn guarded(
    state: &AppState,
    ip: &str,
    work: impl Future<Output = ApiResult<Response>>,
) -> ApiResult<Response> {
    let attempt = state
        .limiter
        .begin(&LINK_IP, ip)
        .map_err(ApiError::rate_limited)?;
    let result = work.await;
    match &result {
        Ok(_) => attempt.succeeded(),
        Err(e) if e.status() == StatusCode::UNAUTHORIZED => attempt.failed(),
        Err(_) => {}
    }
    result
}

/// Consumes a token of `purpose`; malformed tokens are rejected without a lookup.
async fn consume(
    tx: &mut AnyConnection,
    purpose: TokenPurpose,
    token: &SecretToken,
) -> DbResult<Option<String>> {
    if token.expose().is_empty() || token.expose().len() > 128 {
        return Ok(None);
    }
    user_token::consume(tx, purpose, token, now_millis()).await
}

/// Scope of the user whose one-time link was just consumed.
#[allow(clippy::disallowed_methods)] // User id comes from a consumed one-time link (scope docs).
fn link_scope(user_id: &str) -> UserScope {
    UserScope::from_session(user_id)
}
