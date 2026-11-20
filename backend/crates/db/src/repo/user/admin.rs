//! User functions of the administration ([`AdminScope`]) and the admin bootstrap.

use sqlx::{AnyConnection, Row};

use super::{NewUser, Role, User, UserPatch, insert, load};
use crate::error::{DbError, DbResult};
use crate::row::{flag_value, now_millis};
use crate::scope::AdminScope;

/// Creates an admin without account (bootstrap when no admin exists).
pub async fn create_admin(
    conn: &mut AnyConnection,
    _admin: &AdminScope,
    new: &NewUser,
) -> DbResult<User> {
    let new = NewUser {
        role: Role::Admin,
        ..new.clone()
    };
    insert(conn, None, &new).await
}

/// Reads any user (admin user management).
pub async fn get_for_admin(
    conn: &mut AnyConnection,
    _admin: &AdminScope,
    id: &str,
) -> DbResult<User> {
    load(conn, id).await
}

/// Updates any user (admin); same semantics as [`super::update`].
pub async fn update_for_admin(
    conn: &mut AnyConnection,
    _admin: &AdminScope,
    id: &str,
    patch: &UserPatch,
) -> DbResult<User> {
    let result = sqlx::query(
        "UPDATE app_user SET name = COALESCE(?, name), role = COALESCE(?, role), \
         locked = COALESCE(?, locked), updated_at = ? WHERE id = ?",
    )
    .bind(patch.name.as_deref())
    .bind(patch.role.map(Role::code))
    .bind(patch.locked.map(flag_value))
    .bind(now_millis())
    .bind(id)
    .execute(&mut *conn)
    .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::NotFound);
    }
    load(conn, id).await
}

/// Whether at least one admin exists (admin bootstrap, #69).
pub async fn admin_exists(conn: &mut AnyConnection) -> DbResult<bool> {
    let sql = "SELECT COUNT(*) AS n FROM app_user WHERE role = 'admin'";
    let n: i64 = sqlx::query(sql).fetch_one(&mut *conn).await?.try_get("n")?;
    Ok(n > 0)
}
