//! `POST /api/auth/login`, `POST /api/auth/logout`, `GET /api/auth/session`.

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Response;
use serde::Deserialize;
use vermieterme_db::repo::user::{self, normalize_email};
use vermieterme_db::{AnyConnection, UserScope};

use super::client_ip::ClientIp;
use super::extract::CurrentUser;
use super::password::{self, Verification};
use super::rate_limit::{LOGIN_EMAIL, LOGIN_IP, ip_key};
use super::session::{self, SessionJson};
use super::timing::Deadline;
use crate::error::json::ApiJson;
use crate::error::validate::{EMAIL_MAX, PASSWORD_MAX};
use crate::error::{ApiError, ApiResult};
use crate::routes::AppState;

/// The one answer to every failed login (contract: generic).
const LOGIN_FAILED: &str = "Anmeldung fehlgeschlagen";

/// Request of `POST /api/auth/login`.
#[derive(Debug, Deserialize)]
pub struct LoginBody {
    email: String,
    password: String,
}

/// Signs in with e-mail and password (public). Every outcome takes at least
/// [`super::timing::MIN_RESPONSE`]; failures are counted per address and per e-mail.
pub async fn login(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    ApiJson(body): ApiJson<LoginBody>,
) -> ApiResult<Response> {
    let deadline = Deadline::start();
    let ip = ip_key(ip);
    let limiter = &state.limiter;
    // An over-long address is never a valid login: refused before it becomes a key.
    let Some(email) = limiter_email(&body.email) else {
        let result = match limiter.begin(&LOGIN_IP, &ip) {
            Err(wait) => Err(ApiError::rate_limited(wait)),
            Ok(by_ip) => {
                by_ip.failed();
                Err(ApiError::unauthenticated(LOGIN_FAILED))
            }
        };
        return deadline.finish(result).await;
    };
    // Both reservations before any work: parallel requests cannot exceed the limits.
    let reserved = limiter
        .begin(&LOGIN_IP, &ip)
        .and_then(|by_ip| Ok((by_ip, limiter.begin(&LOGIN_EMAIL, &email)?)));
    let result = match reserved {
        Err(wait) => Err(ApiError::rate_limited(wait)),
        Ok((by_ip, by_email)) => {
            let result = attempt(&state, &headers, &email, body.password).await;
            match &result {
                Ok(_) => {
                    by_ip.succeeded();
                    by_email.succeeded();
                }
                Err(e) if e.status() == axum::http::StatusCode::UNAUTHORIZED => {
                    by_ip.failed();
                    by_email.failed();
                }
                Err(_) => {}
            }
            result
        }
    };
    deadline.finish(result).await
}

/// The normalised e-mail as rate-limit key; `None` if longer than [`EMAIL_MAX`] (checked
/// before normalising, so a huge body is never copied into the limiter).
fn limiter_email(email: &str) -> Option<String> {
    let trimmed = email.trim();
    (trimmed.chars().count() <= EMAIL_MAX).then(|| normalize_email(trimmed))
}

/// Verifies the credentials and starts the session; any mismatch -> generic 401.
async fn attempt(
    state: &AppState,
    headers: &HeaderMap,
    email: &str,
    password: String,
) -> ApiResult<Response> {
    if password.chars().count() > PASSWORD_MAX {
        return Err(ApiError::unauthenticated(LOGIN_FAILED));
    }
    let mut conn = state.conn().await?;
    let record = user::find_login(&mut conn, email).await?;
    let stored = record.as_ref().and_then(|r| r.password_hash.clone());
    let verification = password::verify(password.clone(), stored).await?;
    let (Some(record), Verification::Valid { rehash }) = (record, verification) else {
        return Err(ApiError::unauthenticated(LOGIN_FAILED));
    };
    let started = session::start(&mut conn, &state.config, headers, &record.user.id)
        .await?
        .ok_or_else(|| ApiError::unauthenticated(LOGIN_FAILED))?;
    if rehash {
        upgrade_hash(&mut conn, &record.user.id, password).await?;
    }
    let document = session::describe(&mut conn, &started.active.user, None).await?;
    Ok(session::signed_in(&started, document, &state.config))
}

/// Replaces a legacy (bcrypt) or outdated hash by argon2id after a successful sign-in.
#[allow(clippy::disallowed_methods)] // The user just proved the password; own row only.
async fn upgrade_hash(conn: &mut AnyConnection, user_id: &str, password: String) -> ApiResult<()> {
    let hash = password::hash(password).await?;
    user::set_password_hash(conn, &UserScope::from_session(user_id), &hash).await?;
    Ok(())
}

/// Ends the current session and deletes the cookie; always `204` (also without session).
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    let mut conn = state.conn().await?;
    session::end_presented(&mut conn, &state.config, &headers).await?;
    Ok(session::signed_out(&state.config))
}

/// The session document of the signed-in user (`401` without session).
pub async fn current(
    State(state): State<AppState>,
    current: CurrentUser,
) -> ApiResult<axum::Json<SessionJson>> {
    let mut conn = state.conn().await?;
    let document = session::describe(
        &mut conn,
        &current.user,
        current.acting_account_id.as_deref(),
    )
    .await?;
    Ok(axum::Json(document))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limiter_email_is_normalised_and_capped() {
        assert_eq!(
            limiter_email(" Max@Example.org ").as_deref(),
            Some("max@example.org")
        );
        let longest = format!("{}@x.de", "a".repeat(EMAIL_MAX - 5));
        assert_eq!(limiter_email(&longest).map(|e| e.len()), Some(EMAIL_MAX));
        assert_eq!(limiter_email(&format!("a{longest}")), None);
        assert_eq!(limiter_email(&"x".repeat(1_000_000)), None);
    }
}
