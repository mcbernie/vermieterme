//! What of a database error may be logged (plan, decision 35): its kind and the
//! SQLSTATE/vendor code, never the driver message – it can quote field values (e.g. the
//! duplicate key of a MySQL unique violation) or table contents.

use std::fmt;

use vermieterme_db::DbError;

/// Loggable diagnosis of a database error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbDiagnosis {
    /// Kind, e.g. `database.foreign_key`, `pool_timed_out`, `io`, `migration`.
    pub kind: &'static str,
    /// SQLSTATE (MySQL/MariaDB) or result code (SQLite), if the database sent one.
    pub code: Option<String>,
}

impl fmt::Display for DbDiagnosis {
    /// `Art: <kind>, Code: <code or ->`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = self.code.as_deref().unwrap_or("-");
        write!(f, "Art: {}, Code: {code}", self.kind)
    }
}

/// Kind and code of `error`.
pub fn diagnose(error: &DbError) -> DbDiagnosis {
    match error {
        DbError::NotFound => plain("not_found"),
        DbError::Conflict => plain("conflict"),
        DbError::Constraint => plain("constraint"),
        DbError::UnsupportedUrl => plain("unsupported_url"),
        DbError::InvalidValue(_) => plain("invalid_value"),
        DbError::Migration(inner) => match inner {
            sqlx::migrate::MigrateError::Execute(e)
            | sqlx::migrate::MigrateError::ExecuteMigration(e, _) => DbDiagnosis {
                kind: "migration",
                code: sqlx_diagnosis(e).code,
            },
            _ => plain("migration"),
        },
        DbError::Sqlx(inner) => sqlx_diagnosis(inner),
    }
}

/// Logs `error` at level `error` with its diagnosis and the fixed `context` message.
pub fn log(error: &DbError, context: &'static str) {
    let d = diagnose(error);
    tracing::error!(kind = d.kind, sqlstate = d.code.as_deref(), "{context}");
}

/// Logs `error` at level `warn` (expected, transient failures such as a health check).
pub fn warn(error: &DbError, context: &'static str) {
    let d = diagnose(error);
    tracing::warn!(kind = d.kind, sqlstate = d.code.as_deref(), "{context}");
}

/// The one line printed when the binary fails (start, health check): the outermost
/// message (our own, generic texts) plus the diagnosis of a database error in the chain.
/// The rest of the chain is left out on purpose: it may hold the driver message.
pub fn failure_line(error: &anyhow::Error) -> String {
    let db = error.chain().find_map(|e| e.downcast_ref::<DbError>());
    match db {
        Some(db) => format!("Fehler: {error} ({})", diagnose(db)),
        None => format!("Fehler: {error}"),
    }
}

/// Diagnosis without code.
fn plain(kind: &'static str) -> DbDiagnosis {
    DbDiagnosis { kind, code: None }
}

/// Kind and code of a driver error.
fn sqlx_diagnosis(error: &sqlx::Error) -> DbDiagnosis {
    use sqlx::error::ErrorKind;
    let kind = match error {
        sqlx::Error::Database(db) => match db.kind() {
            ErrorKind::UniqueViolation => "database.unique",
            ErrorKind::ForeignKeyViolation => "database.foreign_key",
            ErrorKind::NotNullViolation => "database.not_null",
            ErrorKind::CheckViolation => "database.check",
            _ => "database",
        },
        sqlx::Error::Configuration(_) => "configuration",
        sqlx::Error::Io(_) => "io",
        sqlx::Error::Tls(_) => "tls",
        sqlx::Error::Protocol(_) => "protocol",
        sqlx::Error::RowNotFound => "row_not_found",
        sqlx::Error::ColumnNotFound(_) | sqlx::Error::ColumnIndexOutOfBounds { .. } => "column",
        sqlx::Error::ColumnDecode { .. } | sqlx::Error::Decode(_) => "decode",
        sqlx::Error::Encode(_) => "encode",
        sqlx::Error::PoolTimedOut => "pool_timed_out",
        sqlx::Error::PoolClosed => "pool_closed",
        sqlx::Error::WorkerCrashed => "worker_crashed",
        sqlx::Error::Migrate(_) => "migration",
        _ => "other",
    };
    let code = match error {
        sqlx::Error::Database(db) => db.code().map(|c| safe_code(&c)),
        _ => None,
    };
    DbDiagnosis { kind, code }
}

/// A code as sent by the database, reduced to at most 16 ASCII alphanumerics (codes are
/// short identifiers; anything else must not slip into the log).
fn safe_code(code: &str) -> String {
    code.chars()
        .filter(char::is_ascii_alphanumeric)
        .take(16)
        .collect()
}

#[cfg(test)]
mod tests;
