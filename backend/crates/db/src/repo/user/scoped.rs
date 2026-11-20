//! User functions within one account ([`AccountScope`]).
//!
//! These functions see and change **members only** (`role = 'member'`). The role `admin`
//! spans all accounts, so an admin who is also member of the account (decision 21) is
//! invisible here and behaves like a foreign id ([`DbError::NotFound`]). The input types
//! carry no role: through an account scope a user is always created as `member`, and
//! role changes exist only in the administration (`update_for_admin`).

use sqlx::AnyConnection;

use super::{MemberPatch, NewMember, Role, User, from_row, insert, load};
use crate::error::{DbError, DbResult};
use crate::repo::session;
use crate::row::{flag_value, now_millis};
use crate::scope::AccountScope;
use crate::tx::in_transaction;

/// Creates a member in the scope's account. Taken address -> [`DbError::Conflict`].
pub async fn create(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    new: &NewMember,
) -> DbResult<User> {
    insert(
        conn,
        Some(scope.account_id()),
        &new.email,
        &new.name,
        Role::Member,
        new.password_hash.as_ref(),
    )
    .await
}

/// Lists the members of the scope's account, ordered by name. The order is byte-exact
/// (both dialects compare binary); locale-aware sorting is up to the user interface.
pub async fn list(conn: &mut AnyConnection, scope: &AccountScope) -> DbResult<Vec<User>> {
    let sql = concat!(
        "SELECT ",
        columns!(),
        " FROM app_user WHERE account_id = ? AND role = 'member' ORDER BY name, id"
    );
    let rows = sqlx::query(sql)
        .bind(scope.account_id())
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(|r| from_row(r).map(|l| l.user)).collect()
}

/// Reads a member of the scope's account; foreign, admin or missing -> [`DbError::NotFound`].
pub async fn get(conn: &mut AnyConnection, scope: &AccountScope, id: &str) -> DbResult<User> {
    let sql = concat!(
        "SELECT ",
        columns!(),
        " FROM app_user WHERE id = ? AND account_id = ? AND role = 'member'"
    );
    let row = sqlx::query(sql)
        .bind(id)
        .bind(scope.account_id())
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(DbError::NotFound)?;
    Ok(from_row(&row)?.user)
}

/// Updates name and/or lock state of a member with one `UPDATE` (only sent fields change).
/// Locking ends all sessions of the member in the same transaction (security baseline).
pub async fn update(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
    patch: &MemberPatch,
) -> DbResult<User> {
    in_transaction(conn, async |tx| {
        let result = sqlx::query(
            "UPDATE app_user SET name = COALESCE(?, name), locked = COALESCE(?, locked), \
             updated_at = ? WHERE id = ? AND account_id = ? AND role = 'member'",
        )
        .bind(patch.name.as_deref())
        .bind(patch.locked.map(flag_value))
        .bind(now_millis())
        .bind(id)
        .bind(scope.account_id())
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound);
        }
        if patch.locked == Some(true) {
            session::delete_for_user(tx, id, None).await?;
        }
        load(tx, id).await
    })
    .await
}

/// Deletes a member of the scope's account (sessions and tokens cascade).
pub async fn delete(conn: &mut AnyConnection, scope: &AccountScope, id: &str) -> DbResult<()> {
    let sql = "DELETE FROM app_user WHERE id = ? AND account_id = ? AND role = 'member'";
    let result = sqlx::query(sql)
        .bind(id)
        .bind(scope.account_id())
        .execute(&mut *conn)
        .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::NotFound);
    }
    Ok(())
}
