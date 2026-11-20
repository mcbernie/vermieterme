//! Answers for requests no route handles, in the contract's error shape (axum's own
//! answers have an empty or plain-text body).

use axum::routing::any;

use super::ApiError;
use crate::routes::AppRouter;

/// Adds the fallbacks to the fully assembled router: unknown paths under `/api/` -> 404
/// `not_found`, a known path with a wrong method -> 405 `method_not_allowed`. Paths
/// outside `/api/` are left to the static export (P12), so no router-wide fallback is
/// set. Call after all areas are merged: the method fallback only reaches routes that
/// exist at that point.
pub fn apply(app: AppRouter) -> AppRouter {
    app.route("/api/{*rest}", any(unknown_path))
        .method_not_allowed_fallback(wrong_method)
}

/// 404 for an unknown API path.
async fn unknown_path() -> ApiError {
    ApiError::not_found("Nicht gefunden")
}

/// 405 for a method the path does not support.
async fn wrong_method() -> ApiError {
    ApiError::method_not_allowed()
}
