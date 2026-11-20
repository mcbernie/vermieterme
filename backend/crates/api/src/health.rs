//! `GET /api/health` (public, frozen contract): `200 {"status":"ok","version":…}` if the
//! database answers, else `503 {"status":"error"}`. Used by the Docker healthcheck and
//! the tenant app (`res.ok` only).

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde_json::json;

use crate::routes::{AppRouter, AppState};

/// Version reported by the health check (crate version, e.g. `2.0.0`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Routes of the area.
pub fn router() -> AppRouter {
    AppRouter::new().route("/api/health", get(health))
}

/// Checks the database with a trivial query.
async fn health(State(state): State<AppState>) -> Response {
    match sqlx::query("SELECT 1").execute(state.db.pool()).await {
        Ok(_) => axum::Json(json!({"status": "ok", "version": VERSION})).into_response(),
        Err(error) => {
            tracing::warn!(error = %vermieterme_db::DbError::from(error), "Health-Check fehlgeschlagen");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                axum::Json(json!({"status": "error"})),
            )
                .into_response()
        }
    }
}
