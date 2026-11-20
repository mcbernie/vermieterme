//! The signed-in user's own row ([`UserScope`]) and the public login lookup.

use sqlx::AnyConnection;

use super::{LoginRecord, User, from_row, load, normalize_email};
use crate::error::{DbError, DbResult};
use crate::row::now_millis;
use crate::scope::UserScope;
use crate::secret::PasswordHash;

/// Reads the signed-in user.
pub async fn get_self(conn: &mut AnyConnection, scope: &UserScope) -> DbResult<User> {
    load(conn, scope.user_id()).await
}

/// Changes the display name of the signed-in user.
pub async fn set_own_name(
    conn: &mut AnyConnection,
    scope: &UserScope,
    name: &str,
) -> DbResult<User> {
    let sql = "UPDATE app_user SET name = ?, updated_at = ? WHERE id = ?";
    let result = sqlx::query(sql)
        .bind(name)
        .bind(now_millis())
        .bind(scope.user_id())
        .execute(&mut *conn)
        .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::NotFound);
    }
    load(conn, scope.user_id()).await
}

/// Stores a new password hash of the signed-in user (password change, rehash to argon2id,
/// accepted invitation or reset link). Ending other sessions is up to the caller.
pub async fn set_password_hash(
    conn: &mut AnyConnection,
    scope: &UserScope,
    hash: &PasswordHash,
) -> DbResult<()> {
    let sql = "UPDATE app_user SET password_hash = ?, updated_at = ? WHERE id = ?";
    let result = sqlx::query(sql)
        .bind(hash.as_str())
        .bind(now_millis())
        .bind(scope.user_id())
        .execute(&mut *conn)
        .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::NotFound);
    }
    Ok(())
}

/// Looks up a user and hash by e-mail for the login check (public, therefore unscoped).
pub async fn find_login(conn: &mut AnyConnection, email: &str) -> DbResult<Option<LoginRecord>> {
    let sql = concat!("SELECT ", columns!(), " FROM app_user WHERE email = ?");
    let row = sqlx::query(sql)
        .bind(normalize_email(email))
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(from_row).transpose()
}
