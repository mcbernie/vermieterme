//! Test databases (feature `testing`): one fresh, migrated database per test.
//!
//! - SQLite: temporary file, always available, never skipped.
//! - MariaDB/MySQL: a new database on the server of `TEST_MYSQL_URL`, dropped afterwards.
//!   Without the variable the test prints `[skip]` to stderr (bypassing libtest's output
//!   capture, so the skip is visible) and returns. Tests using it are named `mysql_*`.
//!
//! Helpers panic on setup errors; they run only in tests.

use std::io::Write as _;
use std::path::PathBuf;

use sqlx::Connection as _;

use crate::pool::Db;

/// Environment variable with the MariaDB/MySQL server URL for tests.
pub const TEST_MYSQL_URL: &str = "TEST_MYSQL_URL";

/// What to clean up when the test database is dropped.
enum Cleanup {
    /// Remove the SQLite file and its WAL/SHM companions.
    SqliteFile(PathBuf),
    /// Drop `database` on the server of `server_url`.
    MySqlDatabase {
        server_url: String,
        database: String,
    },
}

/// A migrated database that is removed again when dropped.
pub struct TestDb {
    db: Db,
    url: String,
    cleanup: Cleanup,
}

impl TestDb {
    /// Fresh SQLite database in a temporary file.
    pub async fn sqlite() -> Self {
        let path =
            std::env::temp_dir().join(format!("vermieterme-test-{}.db", uuid::Uuid::now_v7()));
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let db = Db::connect_and_migrate(&url)
            .await
            .unwrap_or_else(|e| panic!("SQLite-Testdatenbank: {e:?}"));
        Self {
            db,
            url,
            cleanup: Cleanup::SqliteFile(path),
        }
    }

    /// Fresh MariaDB/MySQL database, or `None` (after printing `[skip] <test>`) if
    /// `TEST_MYSQL_URL` is not set.
    pub async fn mysql(test: &str) -> Option<Self> {
        let Some(server_url) = std::env::var(TEST_MYSQL_URL).ok().filter(|u| !u.is_empty()) else {
            // One write call, so parallel tests do not interleave inside the line.
            let line = format!("[skip] {test}: {TEST_MYSQL_URL} nicht gesetzt\n");
            let _ = std::io::stderr().write_all(line.as_bytes());
            return None;
        };
        let database = format!("vm_test_{}", uuid::Uuid::now_v7().simple());
        let sql = format!("CREATE DATABASE {database} CHARACTER SET utf8mb4 COLLATE utf8mb4_bin");
        execute_on_server(&server_url, sql).await;
        let url = with_database(&server_url, &database);
        let db = Db::connect_and_migrate(&url)
            .await
            .unwrap_or_else(|e| panic!("MySQL-Testdatenbank: {e:?}"));
        Some(Self {
            db,
            url,
            cleanup: Cleanup::MySqlDatabase {
                server_url,
                database,
            },
        })
    }

    /// The open database.
    pub fn db(&self) -> &Db {
        &self.db
    }

    /// Connection URL of this database (contains credentials for MySQL; tests only).
    pub fn url(&self) -> &str {
        &self.url
    }
}

impl Drop for TestDb {
    /// Removes the database. MySQL needs async IO, so the drop runs on its own thread with
    /// its own runtime (works from inside the test's runtime).
    fn drop(&mut self) {
        match &self.cleanup {
            Cleanup::SqliteFile(path) => {
                for suffix in ["", "-wal", "-shm"] {
                    let mut file = path.clone().into_os_string();
                    file.push(suffix);
                    let _ = std::fs::remove_file(file);
                }
            }
            Cleanup::MySqlDatabase {
                server_url,
                database,
            } => {
                let server_url = server_url.clone();
                let sql = format!("DROP DATABASE IF EXISTS {database}");
                let handle = std::thread::spawn(move || {
                    if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        rt.block_on(execute_on_server(&server_url, sql));
                    }
                });
                let _ = handle.join();
            }
        }
    }
}

/// Runs one administrative statement (CREATE/DROP DATABASE) on the server. The database
/// name is generated (`vm_test_<uuid>`), never input, so assembling the SQL is safe.
async fn execute_on_server(server_url: &str, sql: String) {
    sqlx::any::install_default_drivers();
    let mut conn = sqlx::AnyConnection::connect(server_url)
        .await
        .unwrap_or_else(|e| panic!("Verbindung zu {TEST_MYSQL_URL} fehlgeschlagen: {e:?}"));
    sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
        .execute(&mut conn)
        .await
        .unwrap_or_else(|e| panic!("Testdatenbank anlegen/löschen: {e:?}"));
    let _ = conn.close().await;
}

/// Replaces the database path of a `mysql://user:pw@host:port[/db][?query]` URL.
pub fn with_database(server_url: &str, database: &str) -> String {
    let (base, query) = match server_url.split_once('?') {
        Some((b, q)) => (b, Some(q)),
        None => (server_url, None),
    };
    let (scheme, rest) = base.split_once("://").unwrap_or(("mysql", base));
    let authority = rest.split('/').next().unwrap_or(rest);
    let mut url = format!("{scheme}://{authority}/{database}");
    if let Some(q) = query {
        url.push('?');
        url.push_str(q);
    }
    url
}

/// Defines a test module `$name` running the async function `$body(&Db)` against both
/// dialects: `$name::sqlite` (always) and `$name::mysql_db` (skipped without
/// `TEST_MYSQL_URL`, matched by `just test-mysql`). The calling crate needs `tokio`.
#[macro_export]
macro_rules! dual_test {
    ($name:ident, $body:ident) => {
        mod $name {
            #[tokio::test]
            async fn sqlite() {
                let test_db = $crate::testing::TestDb::sqlite().await;
                super::$body(test_db.db()).await;
            }

            #[tokio::test]
            async fn mysql_db() {
                if let Some(test_db) = $crate::testing::TestDb::mysql(module_path!()).await {
                    super::$body(test_db.db()).await;
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::with_database;

    #[test]
    fn replaces_or_adds_database() {
        assert_eq!(
            with_database("mysql://u:p@h:3306", "x"),
            "mysql://u:p@h:3306/x"
        );
        assert_eq!(with_database("mysql://u:p@h/old", "x"), "mysql://u:p@h/x");
        assert_eq!(
            with_database("mysql://u:p@h/old?ssl-mode=disabled", "x"),
            "mysql://u:p@h/x?ssl-mode=disabled"
        );
    }
}
