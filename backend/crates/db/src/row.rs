//! Small helpers shared by all repositories: ids, timestamps and portable column reads.

use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::Row;
use sqlx::any::AnyRow;

use crate::error::DbResult;

/// New primary key: UUID v7 as text (time-ordered; legacy cuids stay as they are).
pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// Current time as Unix milliseconds UTC (the stored form of all timestamps).
pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// Reads a 0/1 flag column. Flags are integers in both dialects because `Any` cannot
/// decode MySQL `TINYINT` or SQLite integers as `bool`.
pub fn flag(row: &AnyRow, column: &str) -> DbResult<bool> {
    Ok(row.try_get::<i64, _>(column)? != 0)
}

/// Encodes a flag for binding (`true` -> 1).
pub fn flag_value(value: bool) -> i64 {
    i64::from(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_uuid_text() {
        let a = new_id();
        assert_eq!(a.len(), 36);
        assert_ne!(a, new_id());
    }

    #[test]
    fn now_is_after_2026() {
        assert!(now_millis() > 1_767_225_600_000);
    }

    #[test]
    fn flag_encoding() {
        assert_eq!(flag_value(true), 1);
        assert_eq!(flag_value(false), 0);
    }
}
