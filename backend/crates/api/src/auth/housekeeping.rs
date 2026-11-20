//! Periodic cleanup: expired and idle sessions in the database, expired rate-limit
//! entries in memory.

use std::time::Duration;

use vermieterme_db::repo::session;
use vermieterme_db::row::now_millis;

use crate::routes::AppState;

/// Interval of the cleanup.
const INTERVAL: Duration = Duration::from_secs(3600);

/// Runs one cleanup; errors are logged (generic text) and retried next time.
pub async fn run_once(state: &AppState) {
    state.limiter.prune();
    let now = now_millis();
    let idle_since = super::session::idle_since(&state.config, now);
    let result = match state.db.pool().acquire().await {
        Ok(mut conn) => session::delete_expired(&mut conn, now, idle_since).await,
        Err(e) => Err(e.into()),
    };
    match result {
        Ok(removed) => tracing::debug!(removed, "Abgelaufene Sitzungen entfernt"),
        Err(error) => tracing::warn!(error = %error, "Aufräumen der Sitzungen fehlgeschlagen"),
    }
}

/// Starts the cleanup task (first run after one interval).
pub fn spawn(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(INTERVAL);
        ticker.tick().await;
        loop {
            ticker.tick().await;
            run_once(&state).await;
        }
    })
}
