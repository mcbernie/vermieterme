//! Users (web sign-in). A user belongs to one account (membership) and has the role
//! `admin` or `member`; an admin created by the bootstrap may have no account.
//! E-mail addresses are stored trimmed and lower-cased (unique, byte-exact in both dialects).
//!
//! Functions are grouped by scope in private submodules – `scoped` (one account), `admin`
//! (administration, bootstrap) and `own` (the signed-in user, login lookup) – and are all
//! re-exported here.

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};

use crate::error::{DbError, DbResult};
use crate::row::{flag, new_id, now_millis};
use crate::secret::PasswordHash;

/// Role of a user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Manages accounts and may switch into them.
    Admin,
    /// Works inside their own account.
    Member,
}

impl Role {
    /// Stored and API code (`admin`, `member`).
    pub const fn code(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Member => "member",
        }
    }

    /// Reads a stored code.
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "admin" => Some(Role::Admin),
            "member" => Some(Role::Member),
            _ => None,
        }
    }
}

/// A user without credentials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    /// Primary key.
    pub id: String,
    /// Account of the membership (`None` only for an admin without own account).
    pub account_id: Option<String>,
    /// Lower-cased e-mail address (login name).
    pub email: String,
    /// Display name.
    pub name: String,
    /// Role.
    pub role: Role,
    /// Locked users cannot sign in.
    pub locked: bool,
    /// Whether a password is set (invited users have none until they accept).
    pub has_password: bool,
    /// Creation time (Unix ms).
    pub created_at: i64,
    /// Last change (Unix ms).
    pub updated_at: i64,
}

/// Data for a new member, created through an account scope (always role `member`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMember {
    /// E-mail address (normalised on insert).
    pub email: String,
    /// Display name.
    pub name: String,
    /// Initial password hash; `None` for an invitation.
    pub password_hash: Option<PasswordHash>,
}

/// Partial update of a member through an account scope; `None` keeps the stored value.
/// Deliberately without role (role changes only via `update_for_admin`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemberPatch {
    /// New display name.
    pub name: Option<String>,
    /// New lock state.
    pub locked: Option<bool>,
}

/// Data for a new user with any role (administration only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewUser {
    /// E-mail address (normalised on insert).
    pub email: String,
    /// Display name.
    pub name: String,
    /// Role.
    pub role: Role,
    /// Initial password hash; `None` for an invitation.
    pub password_hash: Option<PasswordHash>,
}

/// Partial update of any user including the role (administration only); `None` keeps
/// the stored value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserPatch {
    /// New display name.
    pub name: Option<String>,
    /// New role.
    pub role: Option<Role>,
    /// New lock state.
    pub locked: Option<bool>,
}

/// User plus stored hash, for the login check only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginRecord {
    /// The user.
    pub user: User,
    /// Stored hash, `None` if no password was set yet.
    pub password_hash: Option<PasswordHash>,
}

/// Columns read by [`from_row`].
macro_rules! columns {
    () => {
        "id, account_id, email, name, role, locked, password_hash, created_at, updated_at"
    };
}

/// Normalises an e-mail address for storage and lookup.
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

/// Maps a row selected with `columns!()`.
fn from_row(row: &AnyRow) -> DbResult<LoginRecord> {
    let role: String = row.try_get("role")?;
    let hash: Option<String> = row.try_get("password_hash")?;
    let user = User {
        id: row.try_get("id")?,
        account_id: row.try_get("account_id")?,
        email: row.try_get("email")?,
        name: row.try_get("name")?,
        role: Role::from_code(&role).ok_or(DbError::InvalidValue("app_user.role"))?,
        locked: flag(row, "locked")?,
        has_password: hash.is_some(),
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    };
    Ok(LoginRecord {
        user,
        password_hash: hash.map(PasswordHash::new),
    })
}

mod admin;
mod own;
mod scoped;

pub use admin::{admin_exists, create_for_admin, get_for_admin, update_for_admin};
pub use own::{find_login, get_self, set_own_name, set_password_hash};
pub use scoped::{create, delete, get, list, update};

/// Inserts a user row and reads it back.
async fn insert(
    conn: &mut AnyConnection,
    account_id: Option<&str>,
    email: &str,
    name: &str,
    role: Role,
    password_hash: Option<&PasswordHash>,
) -> DbResult<User> {
    let id = new_id();
    let now = now_millis();
    sqlx::query(
        "INSERT INTO app_user (id, account_id, email, name, password_hash, role, locked, \
         created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?)",
    )
    .bind(&id)
    .bind(account_id)
    .bind(normalize_email(email))
    .bind(name)
    .bind(password_hash.map(PasswordHash::as_str))
    .bind(role.code())
    .bind(now)
    .bind(now)
    .execute(&mut *conn)
    .await?;
    load(conn, &id).await
}

/// Loads one user by id.
async fn load(conn: &mut AnyConnection, id: &str) -> DbResult<User> {
    let sql = concat!("SELECT ", columns!(), " FROM app_user WHERE id = ?");
    let row = sqlx::query(sql)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(DbError::NotFound)?;
    Ok(from_row(&row)?.user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_codes_round_trip() {
        for role in [Role::Admin, Role::Member] {
            assert_eq!(Role::from_code(role.code()), Some(role));
        }
        assert_eq!(Role::from_code("root"), None);
    }

    #[test]
    fn email_is_normalised() {
        assert_eq!(normalize_email("  Max@Example.COM "), "max@example.com");
    }
}
