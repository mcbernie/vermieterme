//! Router assembly: merges the routers of all area modules into one.
//!
//! Each work package activates only its own `merge` line below (and its `mod`
//! declaration in `lib.rs`). Each area module exposes `pub fn router() -> AppRouter`.

use std::sync::Arc;

use axum::Router;
use sqlx::Any;
use sqlx::pool::PoolConnection;
use vermieterme_db::{Db, DbError};

use crate::auth::password::Hasher;
use crate::auth::rate_limit::RateLimiter;
use crate::config::Config;
use crate::error::{ApiError, ApiResult};

/// Shared application state, cloned into every handler (all fields are cheap handles).
#[derive(Debug, Clone)]
pub struct AppState {
    /// Database pool and dialect.
    pub db: Db,
    /// Validated configuration.
    pub config: Arc<Config>,
    /// In-memory rate limits (login, links; decision 13).
    pub limiter: Arc<RateLimiter>,
    /// Bounded password hashing (`PASSWORD_HASH_CONCURRENCY`).
    pub hasher: Hasher,
    /// The one HTTP client for outgoing requests (VPI retrieval); `None` if the TLS
    /// backend could not be initialised (the retrieval then answers 502).
    pub http: Option<reqwest::Client>,
}

/// The shared client of outgoing requests: time limit, no redirects.
fn vpi_client() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .ok()
}

impl AppState {
    /// State over an open (migrated) database.
    pub fn new(db: Db, config: Config) -> Self {
        Self {
            db,
            hasher: Hasher::new(config.password_hash_concurrency),
            config: Arc::new(config),
            limiter: Arc::new(RateLimiter::default()),
            http: vpi_client(),
        }
    }

    /// A pooled connection for the repositories (`&mut *conn` is an `AnyConnection`).
    pub async fn conn(&self) -> ApiResult<PoolConnection<Any>> {
        self.db
            .pool()
            .acquire()
            .await
            .map_err(|e| ApiError::from(DbError::from(e)))
    }
}

/// Router type every area module returns.
pub type AppRouter = Router<AppState>;

/// Builds the complete application router from all activated areas, with the global
/// layers (CSRF check, security headers, request trace).
pub fn router(state: AppState) -> Router {
    let app: AppRouter = Router::new();
    let app = app.merge(crate::auth::router()); // P3
    let app = app.merge(crate::admin::router()); // P3
    let app = app.merge(crate::users::router()); // P3
    let app = app.merge(crate::health::router()); // P3
    let app = app.merge(crate::masterdata::router()); // P4
    // let app = app.merge(crate::billing::router());       // P5
    // let app = app.merge(crate::documents::router());     // P6
    // let app = app.merge(crate::tenant_app::router());    // P7
    // let app = app.merge(crate::pdf::router());           // P8
    // let app = app.merge(crate::backup::router());        // P10
    // let app = app.merge(crate::static_files::router());  // P12
    crate::auth::layers::apply(app, &state).with_state(state)
}

#[cfg(test)]
mod tests {
    use super::{AppState, router};
    use crate::config::Config;
    use vermieterme_db::testing::TestDb;

    #[tokio::test]
    async fn router_builds() {
        let db = TestDb::sqlite().await;
        let config = Config::from_lookup(|name| {
            (name == "AUTH_SECRET").then(|| "0123456789abcdef0123456789abcdef".to_owned())
        })
        .unwrap();
        let _ = router(AppState::new(db.db().clone(), config));
    }
}
