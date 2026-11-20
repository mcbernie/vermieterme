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
use vermieterme_db::{AnyConnection, DbError, DbResult, SecretToken, UserScope, in_transaction};

use super::client_ip::ClientIp;
use super::extract::millis;
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

/// Makes all open links of `user_id` unusable (password change, lock). `issue` removes
/// the unused links of a purpose and stores a new one; the new one expires at once and
/// its secret is dropped, so nothing of the purpose stays redeemable. Run it inside the
/// transaction of the change.
pub async fn revoke(conn: &mut AnyConnection, user_id: &str) -> DbResult<()> {
    let now = now_millis();
    for purpose in [TokenPurpose::Invite, TokenPurpose::Reset] {
        user_token::issue(conn, user_id, purpose, &SecretToken::generate(), now, now).await?;
    }
    Ok(())
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
        let link = Link {
            purpose: TokenPurpose::Invite,
            token: &body.token,
            password: body.password,
        };
        // The session starts inside the transaction: if it is refused (user or account
        // locked), everything rolls back and the invitation stays unused.
        let started = redeem(&state, link, async |tx, scope| {
            user::set_own_name(tx, scope, &name).await?;
            session::start(tx, &state.config, &headers, scope.user_id())
                .await?
                .ok_or(DbError::NotFound)
        })
        .await?;
        let mut conn = state.conn().await?;
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
        let link = Link {
            purpose: TokenPurpose::Reset,
            token: &body.token,
            password: body.password,
        };
        redeem(&state, link, async |tx, scope| {
            session_repo::delete_for_user(tx, scope.user_id(), None).await
        })
        .await?;
        Ok(StatusCode::NO_CONTENT.into_response())
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

/// A presented link: purpose, token as sent by the client, new password.
struct Link<'a> {
    purpose: TokenPurpose,
    token: &'a str,
    password: String,
}

/// Redeems `link`: sets the new password and runs `apply` for the user, returning its
/// value. An invalid link, a locked user or `NotFound` from `apply` -> 401 (generic).
///
/// argon2 runs outside any transaction and without a pool connection (#102, #103): under
/// SQLite a transaction holds the write lock of the whole database. So the link is first
/// checked ([`check`]); only a valid one is hashed (a made-up token costs no argon2 work);
/// then one short transaction consumes the token atomically – failing if it was used in
/// between –, refuses a locked user again, stores the hash and runs `apply`. Any failure
/// rolls back, so an internal error does not burn the token.
async fn redeem<T>(
    state: &AppState,
    link: Link<'_>,
    apply: impl AsyncFnOnce(&mut AnyConnection, &UserScope) -> DbResult<T>,
) -> ApiResult<T> {
    let invalid = || ApiError::unauthenticated(LINK_INVALID);
    let token = SecretToken::from_client(link.token.trim());
    if !check(state, link.purpose, &token).await? {
        return Err(invalid());
    }
    let hash = state.hasher.hash(link.password).await?;
    let mut conn = state.conn().await?;
    let result = in_transaction(&mut conn, async |tx| {
        let scope = consume_for_active_user(tx, link.purpose, &token).await?;
        user::set_password_hash(tx, &scope, &hash).await?;
        apply(tx, &scope).await
    })
    .await;
    result.map_err(|error| match error {
        DbError::NotFound => invalid(),
        other => ApiError::from(other),
    })
}

/// Whether `token` is currently redeemable: valid, unused, and its user not locked. It is
/// consumed in a transaction that is always rolled back (the repository has only the
/// atomic consume, so validity is defined in one place); the transaction is short and
/// computes nothing.
async fn check(state: &AppState, purpose: TokenPurpose, token: &SecretToken) -> ApiResult<bool> {
    let mut conn = state.conn().await?;
    let mut valid = false;
    let result = in_transaction(&mut conn, async |tx| {
        consume_for_active_user(tx, purpose, token).await?;
        valid = true;
        // `NotFound` rolls back like every error; `valid` tells the two cases apart.
        Err::<(), _>(DbError::NotFound)
    })
    .await;
    match result {
        Err(DbError::NotFound) | Ok(()) => Ok(valid),
        Err(other) => Err(ApiError::from(other)),
    }
}

/// Consumes `token` and returns the scope of its user; `NotFound` if the token is not
/// redeemable or the user is locked.
async fn consume_for_active_user(
    tx: &mut AnyConnection,
    purpose: TokenPurpose,
    token: &SecretToken,
) -> DbResult<UserScope> {
    let user_id = consume(tx, purpose, token)
        .await?
        .ok_or(DbError::NotFound)?;
    let scope = link_scope(&user_id);
    if user::get_self(tx, &scope).await?.locked {
        return Err(DbError::NotFound);
    }
    Ok(scope)
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
