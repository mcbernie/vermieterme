//! Migrations embedded in the binary, one set per dialect.
//!
//! Both directories (`backend/migrations/sqlite`, `backend/migrations/mysql`) use the same
//! numbers; numbers are assigned by the plan (0001 P2, 0002 P5, 0003 P7, 0004 P9). A new
//! migration is added to both lists below.

use std::borrow::Cow;

use sqlx::SqlSafeStr;
use sqlx::migrate::{Migration, MigrationType, Migrator};

use crate::dialect::Dialect;

/// One migration in both dialects.
struct Source {
    version: i64,
    description: &'static str,
    sqlite: &'static str,
    mysql: &'static str,
}

/// All migrations in order.
const SOURCES: &[Source] = &[Source {
    version: 1,
    description: "initial",
    sqlite: include_str!("../../../migrations/sqlite/0001_initial.sql"),
    mysql: include_str!("../../../migrations/mysql/0001_initial.sql"),
}];

/// Builds the migrator of `dialect` from the embedded SQL files.
pub fn migrator(dialect: Dialect) -> Migrator {
    let migrations = SOURCES
        .iter()
        .map(|s| {
            Migration::new(
                s.version,
                Cow::Borrowed(s.description),
                MigrationType::Simple,
                dialect.pick(s.sqlite, s.mysql).into_sql_str(),
                false,
            )
        })
        .collect();
    Migrator::with_migrations(migrations)
}

/// Versions of all embedded migrations (for tests and diagnostics).
pub fn versions() -> Vec<i64> {
    SOURCES.iter().map(|s| s.version).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_strictly_increasing_from_one() {
        let v = versions();
        assert_eq!(v.first(), Some(&1));
        assert!(v.windows(2).all(|w| w[1] == w[0] + 1));
    }

    #[test]
    fn every_migration_exists_in_both_dialects() {
        for s in SOURCES {
            assert!(s.sqlite.contains("CREATE TABLE"), "sqlite {}", s.version);
            assert!(s.mysql.contains("CREATE TABLE"), "mysql {}", s.version);
            assert_eq!(
                s.sqlite.matches("CREATE TABLE").count(),
                s.mysql.matches("CREATE TABLE").count(),
                "table count differs in migration {}",
                s.version
            );
        }
    }

    #[test]
    fn migration_files_on_disk_are_embedded() {
        // Every NNNN_*.sql in either directory must be listed in SOURCES (same numbers).
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../migrations");
        for dir in ["sqlite", "mysql"] {
            let mut numbers: Vec<i64> = std::fs::read_dir(format!("{root}/{dir}"))
                .unwrap()
                .filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| n.ends_with(".sql"))
                .map(|n| n[..4].parse().unwrap())
                .collect();
            numbers.sort();
            assert_eq!(numbers, versions(), "directory {dir}");
        }
    }
}
