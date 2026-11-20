//! Data access of vermieterme: connection pool, SQL dialect, migrations and
//! repositories.
//!
//! One code path for SQLite and MySQL/MariaDB through the sqlx `Any` driver; the dialect
//! comes from `DATABASE_URL`, and the few statements that differ are chosen with
//! [`Dialect::pick`] (findings of the `Any` spike: see [`dialect`]).
//!
//! Every repository function with account (customer) reference takes an explicit
//! scope parameter ([`scope`]), so access to foreign data is structurally impossible:
//! a foreign id behaves like a missing one ([`DbError::NotFound`]).

pub mod dialect;
mod error;
mod migrate;
mod pool;
pub mod repo;
pub mod row;
pub mod scope;
pub mod secret;
#[cfg(feature = "testing")]
pub mod testing;

pub use dialect::Dialect;
pub use error::{DbError, DbResult};
pub use migrate::versions as migration_versions;
pub use pool::Db;
pub use scope::{AccountScope, AdminScope, TenantScope, UserScope};
pub use secret::{PasswordHash, SecretToken, TokenHash};
pub use sqlx::AnyConnection;
