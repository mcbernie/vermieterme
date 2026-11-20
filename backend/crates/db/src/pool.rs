//! Connection pool for SQLite or MySQL/MariaDB, selected by `DATABASE_URL`.

use sqlx::AnyPool;
use sqlx::any::AnyPoolOptions;

use crate::dialect::Dialect;
use crate::error::DbResult;
use crate::migrate;

/// Pool size for SQLite: one file, writers serialise anyway; a few readers suffice.
const SQLITE_MAX_CONNECTIONS: u32 = 4;
/// Pool size for MySQL/MariaDB (one application instance, decision 13).
const MYSQL_MAX_CONNECTIONS: u32 = 10;
/// Adds strict mode to the session's `sql_mode`, keeping what sqlx and the server set.
const STRICT_MODE: &str = "SET SESSION sql_mode = CONCAT(@@SESSION.sql_mode, ',STRICT_ALL_TABLES')";

/// Open database: the `Any` pool and the dialect it talks to.
#[derive(Debug, Clone)]
pub struct Db {
    pool: AnyPool,
    dialect: Dialect,
}

impl Db {
    /// Connects to `url` (`sqlite:…` or `mysql:…`). Does not run migrations.
    ///
    /// For SQLite the URL should contain `mode=rwc` to create a missing file. The
    /// journal is switched to WAL so readers do not block the writer; foreign keys are
    /// enforced by sqlx' SQLite defaults.
    ///
    /// Every MySQL connection adds `STRICT_ALL_TABLES` to its `sql_mode`, so a too long
    /// value is rejected (like the SQLite length checks) instead of silently truncated,
    /// whatever the server default is.
    pub async fn connect(url: &str) -> DbResult<Self> {
        let dialect = Dialect::from_url(url)?;
        sqlx::any::install_default_drivers();
        let options = match dialect {
            Dialect::Sqlite => AnyPoolOptions::new().max_connections(SQLITE_MAX_CONNECTIONS),
            Dialect::MySql => AnyPoolOptions::new()
                .max_connections(MYSQL_MAX_CONNECTIONS)
                .after_connect(|conn, _meta| {
                    Box::pin(async move {
                        sqlx::query(STRICT_MODE).execute(&mut *conn).await?;
                        Ok(())
                    })
                }),
        };
        let pool = options.connect(url).await?;
        if dialect == Dialect::Sqlite {
            sqlx::query("PRAGMA journal_mode = WAL")
                .execute(&pool)
                .await?;
        }
        Ok(Self { pool, dialect })
    }

    /// Connects and applies all pending migrations of the dialect.
    pub async fn connect_and_migrate(url: &str) -> DbResult<Self> {
        let db = Self::connect(url).await?;
        db.migrate().await?;
        Ok(db)
    }

    /// Applies all pending migrations (embedded in the binary).
    pub async fn migrate(&self) -> DbResult<()> {
        migrate::migrator(self.dialect).run(&self.pool).await?;
        Ok(())
    }

    /// The pool; repositories take `&mut AnyConnection` from `acquire()` or a transaction.
    pub fn pool(&self) -> &AnyPool {
        &self.pool
    }

    /// The dialect of this database.
    pub fn dialect(&self) -> Dialect {
        self.dialect
    }

    /// Closes all connections (waits for checked-out ones).
    pub async fn close(&self) {
        self.pool.close().await;
    }
}
