//! Tests of the database error diagnosis: kind and code are logged, driver texts never.

use std::io::Write;
use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::MakeWriter;
use vermieterme_db::testing::TestDb;

use super::*;
use crate::error::ApiError;

/// Name in the failing statement; must never show up in a log line.
const MARKER: &str = "geheim_tabelle";

/// Log output collected in memory.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Captured {
    type Writer = Captured;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// A driver error whose message names [`MARKER`] (SQLite: "no such table").
async fn failing_query(db: &TestDb) -> DbError {
    let error = sqlx::query("SELECT * FROM geheim_tabelle")
        .execute(db.db().pool())
        .await
        .unwrap_err();
    assert!(error.to_string().contains(MARKER), "precondition: {error}");
    DbError::from(error)
}

#[tokio::test]
async fn driver_errors_give_kind_and_code() {
    let db = TestDb::sqlite().await;
    let error = failing_query(&db).await;
    let d = diagnose(&error);
    assert_eq!(d.kind, "database");
    assert_eq!(d.code.as_deref(), Some("1"));

    let fk = sqlx::query(
        "INSERT INTO user_token (id, user_id, purpose, token_hash, expires_at, used_at, \
         created_at) VALUES ('t', 'fehlt', 'reset', 'h', 0, NULL, 0)",
    )
    .execute(db.db().pool())
    .await
    .unwrap_err();
    let d = diagnose(&DbError::from(fk));
    assert_eq!(d.kind, "database.foreign_key");
    assert_eq!(d.code.as_deref(), Some("787"));
    assert_eq!(d.to_string(), "Art: database.foreign_key, Code: 787");
}

#[test]
fn errors_without_driver_have_a_kind() {
    assert_eq!(
        diagnose(&DbError::Sqlx(sqlx::Error::PoolTimedOut)).kind,
        "pool_timed_out"
    );
    assert_eq!(diagnose(&DbError::NotFound), plain("not_found"));
    assert_eq!(safe_code("23000\n x"), "23000x");
}

#[tokio::test]
async fn log_line_has_code_but_no_driver_text() {
    let db = TestDb::sqlite().await;
    let error = failing_query(&db).await;
    let captured = Captured::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(captured.clone())
        .with_ansi(false)
        .finish();
    tracing::subscriber::with_default(subscriber, || {
        let api = ApiError::from(error);
        assert_eq!(api.code(), crate::error::ErrorCode::Internal);
    });
    let line = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    assert!(line.contains("Datenbankfehler"), "{line}");
    assert!(line.contains("kind=\"database\""), "{line}");
    assert!(line.contains("sqlstate=\"1\""), "{line}");
    assert!(!line.contains(MARKER), "{line}");
}

#[tokio::test]
async fn failure_line_omits_the_chain() {
    let db = TestDb::sqlite().await;
    let error = anyhow::Error::from(failing_query(&db).await).context("Datenbank nicht erreichbar");
    assert!(
        format!("{error:?}").contains(MARKER),
        "precondition: chain holds the text"
    );
    let line = failure_line(&error);
    assert_eq!(
        line,
        "Fehler: Datenbank nicht erreichbar (Art: database, Code: 1)"
    );
    let plain = failure_line(&anyhow::anyhow!("Port belegt"));
    assert_eq!(plain, "Fehler: Port belegt");
}
