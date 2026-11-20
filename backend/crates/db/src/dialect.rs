//! SQL dialect of the configured database and the few places where SQL must differ.
//!
//! # Findings of the `Any` spike (sqlx 0.9, SQLite 3, MariaDB 10.11 and MySQL 8.4)
//!
//! Verified by `tests/any_spike.rs` against all three databases:
//!
//! - **Placeholders:** `?` works for both. `LIMIT ?` binds fine.
//! - **Types:** `Any` decodes only bool, integers, floats, text and blobs. MySQL `DATE`,
//!   `DATETIME`, `DECIMAL` and `TINYINT` are rejected ("Any driver does not support MySql
//!   type"), and SQLite integers never decode as `bool`. Hence the schema uses text dates
//!   (`CHAR(10)`), `BIGINT` milliseconds, and `SMALLINT` 0/1 for flags (read as `i64`,
//!   see [`crate::row::flag`]). No date branching is needed.
//! - **Aggregates:** MySQL `SUM(bigint)` returns `DECIMAL`; write
//!   `CAST(COALESCE(SUM(x), 0) AS SIGNED)` – SQLite accepts the same cast (numeric
//!   affinity) and returns an integer. `COUNT(*)` needs no cast.
//! - **RETURNING:** not available in MySQL (MariaDB only for INSERT/DELETE). Repositories
//!   generate ids in Rust (UUID v7) and read the row back; no branching.
//! - **Upsert:** differs (`ON CONFLICT (...) DO UPDATE SET c = excluded.c` vs.
//!   `ON DUPLICATE KEY UPDATE c = VALUES(c)`) – the only place that needs [`Dialect::pick`].
//!   `VALUES(c)` is deliberate: MySQL 8.4 accepts it with warning 1287 (deprecated), but
//!   the replacement alias form (`... AS new ON DUPLICATE KEY UPDATE c = new.c`) does not
//!   exist in MariaDB 10.11, so `VALUES()` is the only form both understand. Should MySQL
//!   remove it, split MySQL and MariaDB into separate [`Dialect`] variants.
//! - **Affected rows:** sqlx enables `CLIENT_FOUND_ROWS` for MySQL, so an `UPDATE` that
//!   matches a row but changes nothing reports 1 like SQLite. Version checks via
//!   `WHERE version = ?` and `rows_affected()` are portable.
//! - **Errors:** unique violations keep their native error inside `Any`;
//!   `is_unique_violation()` works for both (see [`crate::DbError`]).
//! - **Partial updates** (#25) without dynamic SQL: `SET c = COALESCE(?, c)` for non-null
//!   columns and `SET c = CASE WHEN ? THEN ? ELSE c END` (flag + value) for nullable ones.
//! - **Collation:** MySQL tables use `utf8mb4_bin` so comparisons are byte-exact like in
//!   SQLite; e-mail addresses are lower-cased before storing.

use crate::error::DbError;

/// Database family behind `DATABASE_URL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// SQLite (file database, default for single installations).
    Sqlite,
    /// MySQL 8 or MariaDB 10.11+.
    MySql,
}

impl Dialect {
    /// Detects the dialect from a connection URL (`sqlite:` or `mysql:`).
    ///
    /// The URL itself never appears in the error (it may contain a password).
    pub fn from_url(url: &str) -> Result<Self, DbError> {
        let scheme = url
            .split(':')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        match scheme.as_str() {
            "sqlite" => Ok(Dialect::Sqlite),
            "mysql" => Ok(Dialect::MySql),
            _ => Err(DbError::UnsupportedUrl),
        }
    }

    /// Detects the dialect of an open `Any` connection from its backend name.
    pub fn of(conn: &sqlx::AnyConnection) -> Result<Self, DbError> {
        match conn.backend_name() {
            "SQLite" => Ok(Dialect::Sqlite),
            "MySQL" => Ok(Dialect::MySql),
            _ => Err(DbError::UnsupportedUrl),
        }
    }

    /// Chooses between the SQLite and the MySQL variant of a statement. Both variants are
    /// static strings, so no SQL is ever assembled from input.
    pub const fn pick(self, sqlite: &'static str, mysql: &'static str) -> &'static str {
        match self {
            Dialect::Sqlite => sqlite,
            Dialect::MySql => mysql,
        }
    }

    /// Directory name of the migrations of this dialect below `backend/migrations/`.
    pub const fn migrations_dir(self) -> &'static str {
        match self {
            Dialect::Sqlite => "sqlite",
            Dialect::MySql => "mysql",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_dialect_from_url() {
        assert_eq!(
            Dialect::from_url("sqlite://data/x.db?mode=rwc").unwrap(),
            Dialect::Sqlite
        );
        assert_eq!(
            Dialect::from_url("sqlite::memory:").unwrap(),
            Dialect::Sqlite
        );
        assert_eq!(
            Dialect::from_url("mysql://u:p@localhost/db").unwrap(),
            Dialect::MySql
        );
        assert_eq!(Dialect::from_url("MySQL://h/db").unwrap(), Dialect::MySql);
    }

    #[test]
    fn rejects_other_urls_without_echoing_them() {
        let err = Dialect::from_url("postgres://user:geheim@h/db").unwrap_err();
        assert!(matches!(err, DbError::UnsupportedUrl));
        assert!(!err.to_string().contains("geheim"));
        assert!(Dialect::from_url("").is_err());
    }

    #[test]
    fn picks_variant() {
        assert_eq!(Dialect::Sqlite.pick("a", "b"), "a");
        assert_eq!(Dialect::MySql.pick("a", "b"), "b");
    }
}
