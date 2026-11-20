//! Master data of an account (contract sections 6 and 9, P4): properties, units,
//! tenants, rent changes, cost categories, landlord (`/api/settings`), PDF template and
//! the global VPI values with retrieval and rent proposals.
//!
//! Every handler takes [`AccountCtx`] (members in their account, admins in the account
//! they work in) and passes `ctx.scope` to the repositories, so foreign ids are 404.
//! Every create, update and delete runs together with its audit entry in one transaction
//! ([`audited`]); the entry is only written when an admin works in a foreign account
//! ([`AccountCtx::audit`]). Writing VPI values is global and therefore admin only.

mod cost_category;
mod fields;
mod landlord;
mod pdf_config;
mod pdf_template;
mod property;
mod rent_change;
mod suggestions;
mod tenant;
mod unit;
mod vpi;
mod vpi_fetch;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use serde::Serialize;
use vermieterme_db::repo::masterdata::Rejection;
use vermieterme_db::{AnyConnection, DbResult, in_transaction};

use crate::auth::AccountCtx;
use crate::error::{ApiError, ApiResult, not_found_as};
use crate::routes::{AppRouter, AppState};

/// Routes of the area.
pub fn router() -> AppRouter {
    AppRouter::new()
        .route(
            "/api/properties",
            get(property::list).post(property::create),
        )
        .route(
            "/api/properties/{id}",
            get(property::detail)
                .put(property::update)
                .delete(property::remove),
        )
        .route("/api/units", get(unit::list).post(unit::create))
        .route("/api/units/{id}", put(unit::update).delete(unit::remove))
        .route("/api/tenants", get(tenant::list).post(tenant::create))
        .route(
            "/api/tenants/{id}",
            put(tenant::update).delete(tenant::remove),
        )
        .route(
            "/api/rent-changes",
            get(rent_change::list).post(rent_change::create),
        )
        .route(
            "/api/rent-changes/{id}",
            put(rent_change::update).delete(rent_change::remove),
        )
        .route(
            "/api/cost-categories",
            get(cost_category::list).post(cost_category::create),
        )
        .route(
            "/api/cost-categories/{id}",
            put(cost_category::update).delete(cost_category::remove),
        )
        .route("/api/settings", get(landlord::get).put(landlord::save))
        .route(
            "/api/pdf-template",
            get(pdf_template::get).put(pdf_template::save),
        )
        .route("/api/vpi", get(vpi::list).post(vpi::save))
        .route("/api/vpi/fetch", post(vpi_fetch::fetch))
        .route("/api/vpi/suggestions", get(suggestions::list))
        .route("/api/vpi/{id}", axum::routing::delete(vpi::remove))
}

/// Runs a change that a business rule may refuse, together with its audit entry, in one
/// transaction. `entity_id` names the changed row for the audit entry; the entry is not
/// written (and nothing is committed) if the change returns a [`Rejection`]. A missing or
/// foreign row becomes 404 with `missing`; a [`Rejection`] becomes 400/409.
pub(super) async fn audited_checked<T>(
    state: &AppState,
    ctx: &AccountCtx,
    (action, entity): (&'static str, &'static str),
    missing: &'static str,
    entity_id: impl FnOnce(&T) -> String,
    change: impl AsyncFnOnce(&mut AnyConnection) -> DbResult<Result<T, Rejection>>,
) -> ApiResult<T> {
    let mut conn = state.conn().await?;
    let outcome = in_transaction(&mut conn, async |tx| {
        let value = match change(tx).await? {
            Ok(value) => value,
            Err(rejection) => return Ok(Err(rejection)),
        };
        ctx.audit(tx, action, entity, Some(&entity_id(&value)))
            .await?;
        Ok(Ok(value))
    })
    .await
    .map_err(not_found_as(missing))?;
    outcome.map_err(refusal)
}

/// Like [`audited_checked`] for changes no business rule can refuse.
pub(super) async fn audited<T>(
    state: &AppState,
    ctx: &AccountCtx,
    names: (&'static str, &'static str),
    missing: &'static str,
    entity_id: impl FnOnce(&T) -> String,
    change: impl AsyncFnOnce(&mut AnyConnection) -> DbResult<T>,
) -> ApiResult<T> {
    audited_checked(state, ctx, names, missing, entity_id, async |tx| {
        change(tx).await.map(Ok)
    })
    .await
}

/// The error answer for a refused change.
fn refusal(rejection: Rejection) -> ApiError {
    match rejection {
        Rejection::SharesAboveTotal => ApiError::field(
            "shares",
            "Die Anteile dürfen die Gesamtanteile des Objekts nicht überschreiten",
        ),
        Rejection::TotalBelowUnitShares => ApiError::field(
            "totalShares",
            "Die Gesamtanteile dürfen nicht kleiner sein als die Anteile einer Einheit",
        ),
        Rejection::CategoryInUse => {
            ApiError::conflict("Die Kostenart wird in Abrechnungen verwendet")
        }
    }
}

/// `201 Created` with the object as JSON.
pub(super) fn created<T: Serialize>(value: T) -> Response {
    (StatusCode::CREATED, axum::Json(value)).into_response()
}

/// `204 No Content` (deleted).
pub(super) fn deleted() -> Response {
    StatusCode::NO_CONTENT.into_response()
}
