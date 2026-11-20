//! User functions within one account ([`AccountScope`]).

use sqlx::AnyConnection;

use super::{NewUser, Role, User, UserPatch, from_row, insert, load};
use crate::error::{DbError, DbResult};
use crate::row::{flag_value, now_millis};
use crate::scope::AccountScope;

/// Creates a user in the scope's account. Taken address -> [`DbError::Conflict`].
pub async fn create(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    new: &NewUser,
) -> DbResult<User> {
    insert(conn, Some(scope.account_id()), new).await
}

/// Lists the users of the scope's account, ordered by name.
pub async fn list(conn: &mut AnyConnection, scope: &AccountScope) -> DbResult<Vec<User>> {
    let sql = concat!(
        "SELECT ",
        columns!(),
        " FROM app_user WHERE account_id = ? ORDER BY name, id"
    );
    let rows = sqlx::query(sql)
        .bind(scope.account_id())
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(|r| from_row(r).map(|l| l.user)).collect()
}

/// Reads a user of the scope's account; foreign or missing -> [`DbError::NotFound`].
pub async fn get(conn: &mut AnyConnection, scope: &AccountScope, id: &str) -> DbResult<User> {
    let sql = concat!(
        "SELECT ",
        columns!(),
        " FROM app_user WHERE id = ? AND account_id = ?"
    );
    let row = sqlx::query(sql)
        .bind(id)
        .bind(scope.account_id())
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(DbError::NotFound)?;
    Ok(from_row(&row)?.user)
}

/// Updates a user of the scope's account with one `UPDATE` (only sent fields change).
pub async fn update(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
    patch: &UserPatch,
) -> DbResult<User> {
    let result = sqlx::query(
        "UPDATE app_user SET name = COALESCE(?, name), role = COALESCE(?, role), \
         locked = COALESCE(?, locked), updated_at = ? WHERE id = ? AND account_id = ?",
    )
    .bind(patch.name.as_deref())
    .bind(patch.role.map(Role::code))
    .bind(patch.locked.map(flag_value))
    .bind(now_millis())
    .bind(id)
    .bind(scope.account_id())
    .execute(&mut *conn)
    .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::NotFound);
    }
    load(conn, id).await
}

/// Deletes a user of the scope's account (sessions and tokens cascade).
pub async fn delete(conn: &mut AnyConnection, scope: &AccountScope, id: &str) -> DbResult<()> {
    let result = sqlx::query("DELETE FROM app_user WHERE id = ? AND account_id = ?")
        .bind(id)
        .bind(scope.account_id())
        .execute(&mut *conn)
        .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::NotFound);
    }
    Ok(())
}
