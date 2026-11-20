//! Session lifecycle (start with rotation, end) and the session document returned by
//! `GET /api/auth/session`, login, invitation and admin switch.

use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use vermieterme_db::repo::account;
use vermieterme_db::repo::session::{self, ActiveSession};
use vermieterme_db::repo::user::User;
use vermieterme_db::row::now_millis;
use vermieterme_db::{AnyConnection, DbResult, SecretToken};

use super::cookie;
use super::extract::{active_account, millis};
use crate::config::Config;
use crate::error::ApiResult;

/// `user` part of the session document.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SessionUserJson {
    /// User id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// E-mail address.
    pub email: String,
    /// `admin` or `member`.
    pub role: &'static str,
}

/// `account` part of the session document.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SessionAccountJson {
    /// Account id.
    pub id: String,
    /// Account name.
    pub name: String,
}

/// Session document of the contract (section 3).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SessionJson {
    /// The signed-in user.
    pub user: SessionUserJson,
    /// The account worked in, `null` for an admin without own or switched account.
    pub account: Option<SessionAccountJson>,
    /// An admin works inside a foreign account.
    pub impersonating: bool,
}

/// Builds the session document for `user` with the switched account `acting`.
pub async fn describe(
    conn: &mut AnyConnection,
    user: &User,
    acting: Option<&str>,
) -> ApiResult<SessionJson> {
    let account = match active_account(user, acting) {
        Some((scope, impersonating)) => {
            let account = account::get_own(conn, &scope).await?;
            Some((
                SessionAccountJson {
                    id: account.id,
                    name: account.name,
                },
                impersonating,
            ))
        }
        None => None,
    };
    Ok(SessionJson {
        user: SessionUserJson {
            id: user.id.clone(),
            name: user.name.clone(),
            email: user.email.clone(),
            role: user.role.code(),
        },
        impersonating: account.as_ref().is_some_and(|(_, i)| *i),
        account: account.map(|(a, _)| a),
    })
}

/// A newly started session: token for the cookie and the verified session row.
#[derive(Debug)]
pub struct Started {
    /// Plain token (only in the cookie).
    pub token: SecretToken,
    /// The session with its user, as the next request will see it.
    pub active: ActiveSession,
}

/// Starts a session for `user_id` after successful authentication. Rotation: the session
/// of a cookie the client still sent is ended first. Returns `None` – and leaves no
/// session behind – if the user or its account is locked (the same check as every later
/// request, `session::find_valid`).
pub async fn start(
    conn: &mut AnyConnection,
    config: &Config,
    headers: &HeaderMap,
    user_id: &str,
) -> DbResult<Option<Started>> {
    end_presented(conn, config, headers).await?;
    let token = SecretToken::generate();
    let now = now_millis();
    let expires_at = now.saturating_add(millis(config.session_max));
    let created = session::create(conn, user_id, &token, now, expires_at).await?;
    match session::find_valid(conn, &token, now, idle_since(config, now)).await? {
        Some(active) => Ok(Some(Started { token, active })),
        None => {
            session::delete(conn, &created.id).await?;
            Ok(None)
        }
    }
}

/// Ends the session of the cookie in `headers`, if it is still valid.
pub async fn end_presented(
    conn: &mut AnyConnection,
    config: &Config,
    headers: &HeaderMap,
) -> DbResult<()> {
    let Some(token) = cookie::read(headers) else {
        return Ok(());
    };
    let now = now_millis();
    if let Some(active) = session::find_valid(conn, &token, now, idle_since(config, now)).await? {
        session::delete(conn, &active.session.id).await?;
    }
    Ok(())
}

/// Earliest `last_seen_at` of a session that is not idle at `now`.
pub(crate) fn idle_since(config: &Config, now: i64) -> i64 {
    now.saturating_sub(millis(config.session_idle))
}

/// `200` with the session document and the cookie of a new session.
pub fn signed_in(started: &Started, document: SessionJson, config: &Config) -> Response {
    let mut response = axum::Json(document).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie::set(&started.token, config.session_max, config.cookie_secure),
    );
    response
}

/// `204` that deletes the cookie.
pub fn signed_out(config: &Config) -> Response {
    let cookie: HeaderValue = cookie::clear(config.cookie_secure);
    (StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie)]).into_response()
}
