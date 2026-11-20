//! `/api/rent-changes`: rent and prepayment changes of a unit (amounts in cents).

use axum::extract::{Path, State};
use axum::response::Response;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use vermieterme_db::repo::masterdata::rent_change::{self, RentChange, RentChangeData, TYPES};
use vermieterme_db::repo::masterdata::unit as unit_repo;
use vermieterme_db::repo::masterdata::unit_ref::{self, UnitRef};
use vermieterme_domain::Cents;

use super::fields::{ID_INPUT_MAX, filter};
use super::unit::NOT_FOUND as UNIT_NOT_FOUND;
use super::unit_embed::{self, UnitEmbed};
use super::{audited, created, deleted};
use crate::auth::AccountCtx;
use crate::error::json::{ApiJson, ApiQuery, timestamp};
use crate::error::validate::Validator;
use crate::error::{ApiResult, not_found_as};
use crate::routes::AppState;

/// Message for a missing or foreign change.
pub const NOT_FOUND: &str = "Mietänderung nicht gefunden";
/// Longest reason (characters).
const REASON_MAX: usize = 2000;

/// A change in responses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RentChangeJson {
    id: String,
    unit_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    unit: Option<UnitEmbed>,
    #[serde(rename = "type")]
    change_type: String,
    amount_cents: i64,
    effective_date: String,
    reason: Option<String>,
    created_at: String,
    updated_at: String,
}

impl RentChangeJson {
    /// Response form with ISO timestamps and the embedded unit with its property.
    pub(super) fn new(r: RentChange, units: &HashMap<String, UnitRef>) -> Self {
        Self {
            id: r.id,
            unit: unit_embed::lookup(units, &r.data.unit_id),
            unit_id: r.data.unit_id,
            change_type: r.data.change_type,
            amount_cents: r.data.amount_cents,
            effective_date: r.data.effective_date,
            reason: r.data.reason,
            created_at: timestamp(r.created_at),
            updated_at: timestamp(r.updated_at),
        }
    }
}

/// Query of the list.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RentChangeQuery {
    unit_id: Option<String>,
}

/// Request of create and replace.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RentChangeBody {
    unit_id: String,
    #[serde(rename = "type")]
    change_type: String,
    amount_cents: i64,
    effective_date: String,
    reason: Option<String>,
}

impl RentChangeBody {
    /// Validated, trimmed values; the amount is between 0 and the cent limit.
    fn validate(&self) -> ApiResult<RentChangeData> {
        let mut v = Validator::new();
        let data = RentChangeData {
            unit_id: v.text("unitId", "Einheit", &self.unit_id, ID_INPUT_MAX),
            change_type: v.code("type", "Art", &self.change_type, &TYPES),
            amount_cents: v.int_in(
                "amountCents",
                "Betrag",
                self.amount_cents,
                (0, Cents::LIMIT),
            ),
            effective_date: v.date("effectiveDate", "Gültig ab", &self.effective_date),
            reason: v.opt_text("reason", "Grund", self.reason.as_deref(), REASON_MAX),
        };
        v.finish()?;
        Ok(data)
    }
}

/// Changes of the account, latest date first, optionally of one unit (a foreign or
/// unknown unit is 404).
pub async fn list(
    State(state): State<AppState>,
    ctx: AccountCtx,
    ApiQuery(query): ApiQuery<RentChangeQuery>,
) -> ApiResult<axum::Json<Vec<RentChangeJson>>> {
    let mut conn = state.conn().await?;
    let unit_id = filter(query.unit_id.as_deref());
    if let Some(unit_id) = unit_id {
        unit_repo::get(&mut conn, &ctx.scope, unit_id)
            .await
            .map_err(not_found_as(UNIT_NOT_FOUND))?;
    }
    let changes = rent_change::list(&mut conn, &ctx.scope, unit_id).await?;
    let units = unit_ref::by_id(&mut conn, &ctx.scope).await?;
    Ok(axum::Json(
        changes
            .into_iter()
            .map(|c| RentChangeJson::new(c, &units))
            .collect(),
    ))
}

/// Creates a change.
pub async fn create(
    State(state): State<AppState>,
    ctx: AccountCtx,
    ApiJson(body): ApiJson<RentChangeBody>,
) -> ApiResult<Response> {
    let data = body.validate()?;
    let change = audited(
        &state,
        &ctx,
        ("rent_change.create", "rent_change"),
        UNIT_NOT_FOUND,
        |r: &RentChange| r.id.clone(),
        async |tx| rent_change::create(tx, &ctx.scope, &data).await,
    )
    .await?;
    let units = unit_embed::load(&state, &ctx).await?;
    Ok(created(RentChangeJson::new(change, &units)))
}

/// Tells a missing change from a missing target unit before the change (the repository
/// re-checks both inside its transaction), so the 404 names the right object.
async fn check_targets(
    state: &AppState,
    ctx: &AccountCtx,
    id: &str,
    unit_id: &str,
) -> ApiResult<()> {
    let mut conn = state.conn().await?;
    rent_change::get(&mut conn, &ctx.scope, id)
        .await
        .map_err(not_found_as(NOT_FOUND))?;
    unit_repo::get(&mut conn, &ctx.scope, unit_id)
        .await
        .map_err(not_found_as(UNIT_NOT_FOUND))?;
    Ok(())
}

/// Replaces a change.
pub async fn update(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
    ApiJson(body): ApiJson<RentChangeBody>,
) -> ApiResult<axum::Json<RentChangeJson>> {
    let data = body.validate()?;
    check_targets(&state, &ctx, &id, &data.unit_id).await?;
    let change = audited(
        &state,
        &ctx,
        ("rent_change.update", "rent_change"),
        NOT_FOUND,
        |r: &RentChange| r.id.clone(),
        async |tx| rent_change::update(tx, &ctx.scope, &id, &data).await,
    )
    .await?;
    let units = unit_embed::load(&state, &ctx).await?;
    Ok(axum::Json(RentChangeJson::new(change, &units)))
}

/// Deletes a change.
pub async fn remove(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    audited(
        &state,
        &ctx,
        ("rent_change.delete", "rent_change"),
        NOT_FOUND,
        |id: &String| id.clone(),
        async |tx| {
            rent_change::delete(tx, &ctx.scope, &id).await?;
            Ok(id.clone())
        },
    )
    .await?;
    Ok(deleted())
}
