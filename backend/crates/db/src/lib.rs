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
//!
//! # Transactions
//!
//! Statements that belong together run through [`in_transaction`], never through
//! `pool().begin()`: it starts SQLite write transactions with `BEGIN IMMEDIATE` (no
//! `SQLITE_BUSY` for "read, then write"), rolls back explicitly and keeps the original
//! error. Inside a transaction it opens a savepoint, so repository functions that use it
//! can be combined in an outer one.
//!
//! # Rules the schema cannot enforce
//!
//! - `billing_period.copied_from_id` has a single-column foreign key (`ON DELETE SET
//!   NULL` cannot null a composite key), so the schema does not stop it from pointing
//!   into another account. It is informational only: the copy function loads the source
//!   period through the account scope and sets the column only to a period of the same
//!   account, and no query follows the link to read data.
//! - Text input is trimmed by the API before it is stored (P3/P4). Trailing spaces
//!   behave differently: MariaDB cuts those beyond a `VARCHAR` limit with a note only
//!   and compares with PAD SPACE (`'a' = 'a  '`), SQLite rejects and compares exactly.
//! - The audit log stores the acting user's id only; the e-mail shown in the log is
//!   joined at read time and becomes `None` once the user is deleted. This is
//!   intentional: the log keeps no copy of personal data.

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
mod tx;

pub use dialect::Dialect;
pub use error::{DbError, DbResult};
pub use migrate::versions as migration_versions;
pub use pool::Db;
pub use scope::{AccountScope, AdminScope, TenantScope, UserScope};
pub use secret::{PasswordHash, SecretToken, TokenHash};
pub use sqlx::AnyConnection;
pub use tx::{in_transaction, in_transaction_read_committed};
