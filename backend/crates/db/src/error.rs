//! The one error type of the data access layer.
//!
//! Messages are German but generic: they never contain SQL, connection URLs or field
//! values (personal data, secrets). The HTTP layer maps the variants to status codes
//! (`NotFound` -> 404, `Conflict` -> 409, everything else -> 500 without details).

/// Error of a database operation.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// The row does not exist **or** belongs to another account (never distinguished,
    /// security baseline: foreign IDs behave like missing ones).
    #[error("Nicht gefunden")]
    NotFound,
    /// A unique key would be violated (e.g. e-mail address already taken).
    #[error("Eintrag existiert bereits")]
    Conflict,
    /// `DATABASE_URL` names no supported database (only `sqlite:` and `mysql:`).
    #[error("Nicht unterstützte Datenbank-URL (erlaubt: sqlite:, mysql:)")]
    UnsupportedUrl,
    /// A stored value cannot be read back (e.g. unknown role code); indicates corrupt data.
    #[error("Ungültiger Wert in Spalte {0}")]
    InvalidValue(&'static str),
    /// Applying the migrations failed.
    #[error("Datenbank-Migration fehlgeschlagen")]
    Migration(#[source] sqlx::migrate::MigrateError),
    /// Any other driver error (connection, syntax, constraint other than unique).
    #[error("Datenbankfehler")]
    Sqlx(#[source] sqlx::Error),
}

impl From<sqlx::Error> for DbError {
    /// Classifies driver errors: missing rows become [`DbError::NotFound`], unique
    /// violations [`DbError::Conflict`]; works through the `Any` driver because it passes
    /// the native database error on unchanged.
    fn from(error: sqlx::Error) -> Self {
        match &error {
            sqlx::Error::RowNotFound => DbError::NotFound,
            sqlx::Error::Database(db) if db.is_unique_violation() => DbError::Conflict,
            _ => DbError::Sqlx(error),
        }
    }
}

impl From<sqlx::migrate::MigrateError> for DbError {
    /// Wraps a migration error.
    fn from(error: sqlx::migrate::MigrateError) -> Self {
        DbError::Migration(error)
    }
}

/// Result alias of the data access layer.
pub type DbResult<T> = Result<T, DbError>;
