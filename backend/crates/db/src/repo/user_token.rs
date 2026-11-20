//! One-time links for users: invitation (`invite`) and password reset (`reset`),
//! decision 22. Only the SHA-256 hash is stored; a token is consumed atomically.

use sqlx::{AnyConnection, Row};

use crate::error::DbResult;
use crate::row::new_id;
use crate::secret::SecretToken;

/// Purpose of a link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenPurpose {
    /// Invitation: the user sets name and first password.
    Invite,
    /// Password reset (24 h).
    Reset,
}

impl TokenPurpose {
    /// Stored code.
    pub const fn code(self) -> &'static str {
        match self {
            TokenPurpose::Invite => "invite",
            TokenPurpose::Reset => "reset",
        }
    }
}

/// Stores a new link for `user_id`; earlier unused links of the same purpose are
/// removed, so only the newest link works.
pub async fn issue(
    conn: &mut AnyConnection,
    user_id: &str,
    purpose: TokenPurpose,
    token: &SecretToken,
    now: i64,
    expires_at: i64,
) -> DbResult<()> {
    sqlx::query("DELETE FROM user_token WHERE user_id = ? AND purpose = ? AND used_at IS NULL")
        .bind(user_id)
        .bind(purpose.code())
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO user_token (id, user_id, purpose, token_hash, expires_at, used_at, \
         created_at) VALUES (?, ?, ?, ?, ?, NULL, ?)",
    )
    .bind(new_id())
    .bind(user_id)
    .bind(purpose.code())
    .bind(token.hash().as_str())
    .bind(expires_at)
    .bind(now)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Consumes a presented link: marks it used if it exists, has the purpose, is unused and
/// not expired, and returns its user id. The conditional `UPDATE` makes concurrent
/// redemptions of the same token succeed at most once.
pub async fn consume(
    conn: &mut AnyConnection,
    purpose: TokenPurpose,
    token: &SecretToken,
    now: i64,
) -> DbResult<Option<String>> {
    let hash = token.hash();
    let result = sqlx::query(
        "UPDATE user_token SET used_at = ? WHERE token_hash = ? AND purpose = ? \
         AND used_at IS NULL AND expires_at > ?",
    )
    .bind(now)
    .bind(hash.as_str())
    .bind(purpose.code())
    .bind(now)
    .execute(&mut *conn)
    .await?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    let row = sqlx::query("SELECT user_id FROM user_token WHERE token_hash = ?")
        .bind(hash.as_str())
        .fetch_one(&mut *conn)
        .await?;
    Ok(Some(row.try_get("user_id")?))
}
