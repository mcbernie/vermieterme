//! Authentication of the web API: sign-in with e-mail and password (argon2id, legacy
//! bcrypt upgraded on sign-in), sessions in the cookie `vm_session` (only the SHA-256 hash
//! is stored, rotation at sign-in), one-time invitation and reset links, CSRF guard,
//! rate limits with equal response time, the permission mapping and the extractors that
//! hand out scopes.
//!
//! Routes (contract section 3):
//! - `POST /api/auth/login` – public
//! - `POST /api/auth/logout` – session (always `204`)
//! - `GET /api/auth/session` – session
//! - `POST /api/auth/accept-invite` – public
//! - `POST /api/auth/reset-password` – public
//!
//! Sign-in paths end in [`session::start`]; a later OIDC sign-in (`user_identity`) only
//! needs to call it with the user id.

pub mod bootstrap;
pub mod client_ip;
pub mod cookie;
mod extract;
pub mod housekeeping;
pub mod layers;
pub mod links;
mod login;
pub mod password;
pub mod permissions;
pub mod rate_limit;
pub mod session;
pub mod timing;

use axum::routing::{get, post};

pub use extract::{AccountCtx, AdminCtx, CurrentUser};

use crate::routes::AppRouter;

/// Routes of the sign-in area.
pub fn router() -> AppRouter {
    AppRouter::new()
        .route("/api/auth/login", post(login::login))
        .route("/api/auth/logout", post(login::logout))
        .route("/api/auth/session", get(login::current))
        .route("/api/auth/accept-invite", post(links::accept_invite))
        .route("/api/auth/reset-password", post(links::reset_password))
}
