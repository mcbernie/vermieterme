//! Accounts (customers, "Kunden"). Managed by admins; members only read their own.

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};

use super::session;
use crate::error::{DbError, DbResult};
use crate::row::{flag, flag_value, new_id, now_millis};
use crate::scope::{AccountScope, AdminScope};
use crate::tx::in_transaction;

/// An account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// Primary key.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Locked accounts cannot sign in (web or tenant app).
    pub locked: bool,
    /// Creation time (Unix ms).
    pub created_at: i64,
    /// Last change (Unix ms).
    pub updated_at: i64,
}

/// An account with its number of users (admin list).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountSummary {
    /// The account.
    pub account: Account,
    /// Number of users belonging to it.
    pub user_count: i64,
}

/// Partial update; `None` keeps the stored value (#25).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccountPatch {
    /// New name.
    pub name: Option<String>,
    /// New lock state.
    pub locked: Option<bool>,
}

/// Column list read by [`from_row`] (a macro, so `concat!` can build static SQL).
macro_rules! columns {
    () => {
        "id, name, locked, created_at, updated_at"
    };
}

/// Maps a row selected with `columns!()`.
fn from_row(row: &AnyRow) -> DbResult<Account> {
    Ok(Account {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        locked: flag(row, "locked")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

/// Creates an account (admin, or the system during bootstrap/import).
pub async fn create(
    conn: &mut AnyConnection,
    _admin: &AdminScope,
    name: &str,
) -> DbResult<Account> {
    let id = new_id();
    let now = now_millis();
    sqlx::query(
        "INSERT INTO account (id, name, locked, created_at, updated_at) VALUES (?, ?, 0, ?, ?)",
    )
    .bind(&id)
    .bind(name)
    .bind(now)
    .bind(now)
    .execute(&mut *conn)
    .await?;
    load(conn, &id).await
}

/// Lists all accounts with their user count, ordered by name. The order is byte-exact
/// (`utf8mb4_bin` / SQLite `BINARY`): upper case before lower case, umlauts after "z".
/// Locale-aware sorting and search over normalised names are up to the caller (sort in
/// Rust or in the user interface).
pub async fn list(conn: &mut AnyConnection, _admin: &AdminScope) -> DbResult<Vec<AccountSummary>> {
    let rows = sqlx::query(
        "SELECT a.id, a.name, a.locked, a.created_at, a.updated_at, \
         (SELECT COUNT(*) FROM app_user u WHERE u.account_id = a.id) AS user_count \
         FROM account a ORDER BY a.name, a.id",
    )
    .fetch_all(&mut *conn)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(AccountSummary {
                account: from_row(row)?,
                user_count: row.try_get("user_count")?,
            })
        })
        .collect()
}

/// Reads any account (admin).
pub async fn get(conn: &mut AnyConnection, _admin: &AdminScope, id: &str) -> DbResult<Account> {
    load(conn, id).await
}

/// Reads the account of the scope (session info of a member).
pub async fn get_own(conn: &mut AnyConnection, scope: &AccountScope) -> DbResult<Account> {
    load(conn, scope.account_id()).await
}

/// Updates name and/or lock state with one `UPDATE` (only sent fields change).
/// Locking ends the sessions of the members in the same transaction, like locking a user
/// ([`session::delete_for_account`]: admins keep theirs but leave the account).
pub async fn update(
    conn: &mut AnyConnection,
    admin: &AdminScope,
    id: &str,
    patch: &AccountPatch,
) -> DbResult<Account> {
    in_transaction(conn, async |tx| {
        let result = sqlx::query(
            "UPDATE account SET name = COALESCE(?, name), locked = COALESCE(?, locked), \
             updated_at = ? WHERE id = ?",
        )
        .bind(patch.name.as_deref())
        .bind(patch.locked.map(flag_value))
        .bind(now_millis())
        .bind(id)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound);
        }
        if patch.locked == Some(true) {
            session::delete_for_account(tx, admin, id).await?;
        }
        load(tx, id).await
    })
    .await
}

/// The first account by creation (owner of the legacy data, decision 21).
pub async fn first(conn: &mut AnyConnection, _admin: &AdminScope) -> DbResult<Option<Account>> {
    let sql = concat!(
        "SELECT ",
        columns!(),
        " FROM account ORDER BY created_at, id LIMIT 1"
    );
    let row = sqlx::query(sql).fetch_optional(&mut *conn).await?;
    row.as_ref().map(from_row).transpose()
}

/// Loads one account by id.
async fn load(conn: &mut AnyConnection, id: &str) -> DbResult<Account> {
    let row = sqlx::query(concat!("SELECT ", columns!(), " FROM account WHERE id = ?"))
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(DbError::NotFound)?;
    from_row(&row)
}
