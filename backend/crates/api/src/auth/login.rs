//! `POST /api/auth/login`, `POST /api/auth/logout`, `GET /api/auth/session`.

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Response;
use serde::Deserialize;
use vermieterme_db::repo::user::{self, normalize_email};
use vermieterme_db::{AnyConnection, PasswordHash, UserScope};

use super::client_ip::ClientIp;
use super::extract::CurrentUser;
use super::password::Verification;
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
    // An over-long address or password is never a valid login: refused before the
    // e-mail becomes a key (a cheap way to create keys otherwise, #101), counted only
    // against the address.
    let email =
        limiter_email(&body.email).filter(|_| body.password.chars().count() <= PASSWORD_MAX);
    let Some(email) = email else {
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
        Ok((mut by_ip, mut by_email)) => {
            // Armed once the password check starts: a cancelled login counts (#104).
            let arm = || {
                by_ip.arm();
                by_email.arm();
            };
            let result = attempt(&state, &headers, &email, body.password, arm).await;
            match &result {
                Ok(_) => {
                    by_ip.succeeded();
                    by_email.succeeded();
                }
                Err(e) if e.status() == axum::http::StatusCode::UNAUTHORIZED => {
                    by_ip.failed();
                    by_email.failed();
                }
                Err(_) => {
                    by_ip.released();
                    by_email.released();
                }
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

/// Verifies the credentials and starts the session; any mismatch -> generic 401. No
/// pool connection is held while waiting for a hash permit or computing (#102): the user
/// is read, the connection returned, the password checked, then a new one is taken.
/// `arm` runs once the hash permit is held, right before the check.
async fn attempt(
    state: &AppState,
    headers: &HeaderMap,
    email: &str,
    password: String,
    arm: impl FnOnce(),
) -> ApiResult<Response> {
    let record = {
        let mut conn = state.conn().await?;
        user::find_login(&mut conn, email).await?
    };
    let stored = record.as_ref().and_then(|r| r.password_hash.clone());
    let permit = state.hasher.permit().await?;
    arm();
    let verification = permit.verify(password.clone(), stored).await?;
    let (Some(record), Verification::Valid { rehash }) = (record, verification) else {
        return Err(ApiError::unauthenticated(LOGIN_FAILED));
    };
    // Computed before the connection is taken; a failure (e.g. busy) must not turn the
    // login into an error – the upgrade is retried at the next sign-in.
    let upgraded = if rehash {
        state.hasher.hash(password).await.ok()
    } else {
        None
    };
    let mut conn = state.conn().await?;
    let started = session::start(&mut conn, &state.config, headers, &record.user.id)
        .await?
        .ok_or_else(|| ApiError::unauthenticated(LOGIN_FAILED))?;
    if rehash {
        store_upgrade(&mut conn, &record.user.id, upgraded.as_ref()).await;
    }
    let document = session::describe(&mut conn, &started.active.user, None).await?;
    Ok(session::signed_in(&started, document, &state.config))
}

/// Stores the argon2id hash replacing a legacy (bcrypt) or outdated hash after a
/// successful sign-in; a missing hash or a failure is only logged (the session exists).
#[allow(clippy::disallowed_methods)] // The user just proved the password; own row only.
async fn store_upgrade(conn: &mut AnyConnection, user_id: &str, hash: Option<&PasswordHash>) {
    let stored = match hash {
        Some(hash) => user::set_password_hash(conn, &UserScope::from_session(user_id), hash)
            .await
            .is_ok(),
        None => false,
    };
    if !stored {
        tracing::warn!("Passwort-Hash konnte nach der Anmeldung nicht erneuert werden");
    }
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
