//! User functions of the administration ([`AdminScope`]) and the admin bootstrap.

use sqlx::{AnyConnection, Row};

use super::{NewUser, Role, User, UserPatch, insert, load};
use crate::error::{DbError, DbResult};
use crate::row::{flag_value, now_millis};
use crate::scope::AdminScope;

/// Creates a user with any role, in `account_id` or without account (admin bootstrap,
/// admin user management, legacy import). Taken address -> [`DbError::Conflict`]; a
/// member without account -> [`DbError::Constraint`] (only admins may have none; the
/// schema enforces the same rule as `app_user_member_account_chk`).
pub async fn create_for_admin(
    conn: &mut AnyConnection,
    _admin: &AdminScope,
    account_id: Option<&str>,
    new: &NewUser,
) -> DbResult<User> {
    if new.role == Role::Member && account_id.is_none() {
        return Err(DbError::Constraint);
    }
    insert(
        conn,
        account_id,
        &new.email,
        &new.name,
        new.role,
        new.password_hash.as_ref(),
    )
    .await
}

/// Reads any user (admin user management).
pub async fn get_for_admin(
    conn: &mut AnyConnection,
    _admin: &AdminScope,
    id: &str,
) -> DbResult<User> {
    load(conn, id).await
}

/// Updates any user including the role (admin); only sent fields change. The only way to
/// change a role. Making a user without account a member is refused by the schema
/// (`app_user_member_account_chk`) -> [`DbError::Constraint`].
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
