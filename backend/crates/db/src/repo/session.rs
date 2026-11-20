//! Web sessions (cookie `vm_session`). Only the SHA-256 hash of the token is stored; the
//! lookup filters expired, idle, locked-user and locked-account sessions in SQL.
//!
//! Durations (idle and maximum age, `SESSION_IDLE_HOURS` / `SESSION_MAX_DAYS`) belong to
//! the HTTP layer's configuration; callers pass the resulting instants in Unix ms.

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};

use crate::error::{DbError, DbResult};
use crate::repo::user::{self, User};
use crate::row::new_id;
use crate::scope::AdminScope;
use crate::secret::SecretToken;

/// A stored session (without the token).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// Primary key (not the token).
    pub id: String,
    /// Signed-in user.
    pub user_id: String,
    /// Account an admin switched into (`None` = own account).
    pub acting_account_id: Option<String>,
    /// Creation time (Unix ms).
    pub created_at: i64,
    /// Last request (Unix ms), for the idle timeout.
    pub last_seen_at: i64,
    /// Absolute end (Unix ms).
    pub expires_at: i64,
}

/// A valid session with its user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveSession {
    /// The session.
    pub session: Session,
    /// The signed-in user (not locked, account not locked).
    pub user: User,
}

/// Session columns of [`find_valid`], aliased where they clash with user columns
/// (`id`, `created_at`); read by [`from_row`].
macro_rules! columns {
    () => {
        "s.id AS session_id, s.user_id, s.acting_account_id, \
         s.created_at AS session_created_at, s.last_seen_at, s.expires_at"
    };
}

/// Maps a row selected with `columns!()`.
fn from_row(row: &AnyRow) -> DbResult<Session> {
    Ok(Session {
        id: row.try_get("session_id")?,
        user_id: row.try_get("user_id")?,
        acting_account_id: row.try_get("acting_account_id")?,
        created_at: row.try_get("session_created_at")?,
        last_seen_at: row.try_get("last_seen_at")?,
        expires_at: row.try_get("expires_at")?,
    })
}

/// Creates a session for an authenticated user; stores only the token hash.
pub async fn create(
    conn: &mut AnyConnection,
    user_id: &str,
    token: &SecretToken,
    now: i64,
    expires_at: i64,
) -> DbResult<Session> {
    let session = Session {
        id: new_id(),
        user_id: user_id.to_owned(),
        acting_account_id: None,
        created_at: now,
        last_seen_at: now,
        expires_at,
    };
    sqlx::query(
        "INSERT INTO web_session (id, token_hash, user_id, acting_account_id, created_at, \
         last_seen_at, expires_at) VALUES (?, ?, ?, NULL, ?, ?, ?)",
    )
    .bind(&session.id)
    .bind(token.hash().as_str())
    .bind(&session.user_id)
    .bind(now)
    .bind(now)
    .bind(expires_at)
    .execute(&mut *conn)
    .await?;
    Ok(session)
}

/// Finds the valid session of a presented token: not expired (`expires_at > now`), used
/// after `idle_since`, user not locked, account not locked (admins are exempt from the
/// account lock). Anything else -> `None`. Session and user come from one statement, so a
/// user deleted concurrently yields `None` (401), never [`DbError::NotFound`] (404).
pub async fn find_valid(
    conn: &mut AnyConnection,
    token: &SecretToken,
    now: i64,
    idle_since: i64,
) -> DbResult<Option<ActiveSession>> {
    let sql = concat!(
        "SELECT ",
        columns!(),
        ", ",
        user::columns!(u),
        " FROM web_session s JOIN app_user u ON u.id = s.user_id \
         LEFT JOIN account a ON a.id = u.account_id \
         WHERE s.token_hash = ? AND s.expires_at > ? AND s.last_seen_at > ? \
         AND u.locked = 0 AND (u.role = 'admin' OR a.locked IS NULL OR a.locked = 0)"
    );
    let row = sqlx::query(sql)
        .bind(token.hash().as_str())
        .bind(now)
        .bind(idle_since)
        .fetch_optional(&mut *conn)
        .await?;
    let Some(row) = row else { return Ok(None) };
    let session = from_row(&row)?;
    let user = user::from_row(&row)?.user;
    Ok(Some(ActiveSession { session, user }))
}

/// Records activity (idle timeout).
pub async fn touch(conn: &mut AnyConnection, session_id: &str, now: i64) -> DbResult<()> {
    sqlx::query("UPDATE web_session SET last_seen_at = ? WHERE id = ?")
        .bind(now)
        .bind(session_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Sets or clears the account an admin works in. Audit logging is up to the caller.
/// Unknown session or unknown account -> [`DbError::NotFound`].
pub async fn set_acting_account(
    conn: &mut AnyConnection,
    _admin: &AdminScope,
    session_id: &str,
    account_id: Option<&str>,
) -> DbResult<()> {
    // The EXISTS turns an unknown account into "no row changed" instead of a foreign key
    // error; `NULL` (back to the own account) always passes.
    let result = sqlx::query(
        "UPDATE web_session SET acting_account_id = ? WHERE id = ? \
         AND (? IS NULL OR EXISTS (SELECT 1 FROM account WHERE id = ?))",
    )
    .bind(account_id)
    .bind(session_id)
    .bind(account_id)
    .bind(account_id)
    .execute(&mut *conn)
    .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::NotFound);
    }
    Ok(())
}

/// Ends one session (logout, rotation at login).
pub async fn delete(conn: &mut AnyConnection, session_id: &str) -> DbResult<()> {
    sqlx::query("DELETE FROM web_session WHERE id = ?")
        .bind(session_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Ends all sessions of a user except `keep` (password change keeps the current one;
/// lock and reset pass `None`). Returns the number of ended sessions.
pub async fn delete_for_user(
    conn: &mut AnyConnection,
    user_id: &str,
    keep: Option<&str>,
) -> DbResult<u64> {
    let result = match keep {
        Some(keep) => {
            sqlx::query("DELETE FROM web_session WHERE user_id = ? AND id <> ?")
                .bind(user_id)
                .bind(keep)
                .execute(&mut *conn)
                .await?
        }
        None => {
            sqlx::query("DELETE FROM web_session WHERE user_id = ?")
                .bind(user_id)
                .execute(&mut *conn)
                .await?
        }
    };
    Ok(result.rows_affected())
}

/// Lock of an account: ends the sessions of its members (admins keep theirs, the account
/// lock does not apply to them) and moves admins who switched into it back to their own
/// account. Returns the number of ended sessions.
pub async fn delete_for_account(
    conn: &mut AnyConnection,
    _admin: &AdminScope,
    account_id: &str,
) -> DbResult<u64> {
    let ended = sqlx::query(
        "DELETE FROM web_session WHERE user_id IN \
         (SELECT id FROM app_user WHERE account_id = ? AND role <> 'admin')",
    )
    .bind(account_id)
    .execute(&mut *conn)
    .await?
    .rows_affected();
    sqlx::query("UPDATE web_session SET acting_account_id = NULL WHERE acting_account_id = ?")
        .bind(account_id)
        .execute(&mut *conn)
        .await?;
    Ok(ended)
}

/// Removes expired and idle sessions (housekeeping). Returns the number removed.
pub async fn delete_expired(conn: &mut AnyConnection, now: i64, idle_since: i64) -> DbResult<u64> {
    let result = sqlx::query("DELETE FROM web_session WHERE expires_at <= ? OR last_seen_at <= ?")
        .bind(now)
        .bind(idle_since)
        .execute(&mut *conn)
        .await?;
    Ok(result.rows_affected())
}
