//! HTTP layer of vermieterme: axum router, handlers, authentication, permission
//! mapping, error shape and configuration.
//!
//! Area modules live at the crate root. Each work package activates exactly its own
//! slot below (and its matching `merge` line in [`routes::router`]); do not reorder
//! or rename the slots.

pub mod routes;

// P3 (kept in alphabetical order: rustfmt sorts adjacent module declarations)
pub mod admin;
pub mod auth;
// P5
pub mod billing;
pub mod config;
// P6
pub mod documents;
pub mod error;
pub mod health;
// P4
pub mod masterdata;
pub mod users;
// pub mod tenant_app;    // P7
// pub mod pdf;           // P8
// pub mod backup;        // P10
// pub mod static_files;  // P12 (serves the static export from STATIC_DIR)
