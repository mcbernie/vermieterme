//! Administration across accounts (contract section 4, #44). Every endpoint needs the
//! role `admin` ([`crate::auth::AdminCtx`]); every change writes an audit entry in the
//! same transaction. Audit entries carry no field values, only action, entity and id.

mod accounts;
mod audit_log;
mod cursor;
mod switch;
mod users;

use axum::routing::{get, patch, post};
use vermieterme_db::AnyConnection;
use vermieterme_db::repo::audit::{self, NewAuditEntry};

use crate::auth::AdminCtx;
use crate::routes::AppRouter;

/// Message for a missing account.
pub const ACCOUNT_NOT_FOUND: &str = "Kunde nicht gefunden";

/// Routes of the area.
pub fn router() -> AppRouter {
    AppRouter::new()
        .route(
            "/api/admin/accounts",
            get(accounts::list).post(accounts::create),
        )
        .route("/api/admin/accounts/{id}", patch(accounts::update))
        .route(
            "/api/admin/accounts/{id}/users",
            get(users::list).post(users::invite),
        )
        .route("/api/admin/users/{id}", patch(users::update))
        .route("/api/admin/users/{id}/reset-link", post(users::reset_link))
        .route(
            "/api/admin/switch",
            post(switch::enter).delete(switch::leave),
        )
        .route("/api/admin/audit-log", get(audit_log::list))
}

/// Writes an audit entry of the admin in `ctx` (in the caller's transaction).
async fn record(
    conn: &mut AnyConnection,
    ctx: &AdminCtx,
    account_id: Option<&str>,
    action: &'static str,
    entity: &'static str,
    entity_id: &str,
) -> vermieterme_db::DbResult<()> {
    let entry = NewAuditEntry {
        user_id: Some(&ctx.user.id),
        account_id,
        action,
        entity,
        entity_id: Some(entity_id),
    };
    audit::record(conn, &entry).await.map(|_| ())
}
