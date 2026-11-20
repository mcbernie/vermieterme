//! Router assembly: merges the routers of all area modules into one.
//!
//! Each work package activates only its own `merge` line below (and its `mod`
//! declaration in `lib.rs`). Each area module exposes `pub fn router() -> AppRouter`.

use axum::Router;

/// Shared application state. Still empty (`()`); P3 replaces this alias by its state
/// struct. Areas return [`AppRouter`], so none of them has to change when it does.
pub type AppState = ();

/// Router type every area module returns.
pub type AppRouter = Router<AppState>;

/// Builds the complete application router from all activated areas.
pub fn router() -> Router {
    let app: AppRouter = Router::new();
    // let app = app.merge(crate::auth::router());          // P3
    // let app = app.merge(crate::masterdata::router());    // P4
    // let app = app.merge(crate::billing::router());       // P5
    // let app = app.merge(crate::documents::router());     // P6
    // let app = app.merge(crate::tenant_app::router());    // P7
    // let app = app.merge(crate::pdf::router());           // P8
    // let app = app.merge(crate::backup::router());        // P10
    // let app = app.merge(crate::static_files::router());  // P12
    app.with_state(())
}

#[cfg(test)]
mod tests {
    use super::router;

    #[test]
    fn router_builds() {
        let _ = router();
    }
}
