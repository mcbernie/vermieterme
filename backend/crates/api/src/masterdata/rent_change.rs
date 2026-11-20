//! `/api/rent-changes`: rent and prepayment changes of a unit (amounts in cents).

use axum::extract::{Path, State};
use axum::response::Response;
use serde::{Deserialize, Serialize};
use vermieterme_db::repo::masterdata::rent_change::{self, RentChange, RentChangeData, TYPES};
use vermieterme_db::repo::masterdata::unit as unit_repo;
use vermieterme_domain::Cents;

use super::unit::NOT_FOUND as UNIT_NOT_FOUND;
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
    #[serde(rename = "type")]
    change_type: String,
    amount_cents: i64,
    effective_date: String,
    reason: Option<String>,
    created_at: String,
    updated_at: String,
}

impl From<RentChange> for RentChangeJson {
    /// Response form with ISO timestamps.
    fn from(r: RentChange) -> Self {
        Self {
            id: r.id,
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
            unit_id: v.text("unitId", "Einheit", &self.unit_id, 36),
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
    if let Some(unit_id) = query.unit_id.as_deref() {
        unit_repo::get(&mut conn, &ctx.scope, unit_id)
            .await
            .map_err(not_found_as(UNIT_NOT_FOUND))?;
    }
    let changes = rent_change::list(&mut conn, &ctx.scope, query.unit_id.as_deref()).await?;
    Ok(axum::Json(
        changes.into_iter().map(RentChangeJson::from).collect(),
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
    Ok(created(RentChangeJson::from(change)))
}

/// Replaces a change.
pub async fn update(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
    ApiJson(body): ApiJson<RentChangeBody>,
) -> ApiResult<axum::Json<RentChangeJson>> {
    let data = body.validate()?;
    let change = audited(
        &state,
        &ctx,
        ("rent_change.update", "rent_change"),
        NOT_FOUND,
        |r: &RentChange| r.id.clone(),
        async |tx| rent_change::update(tx, &ctx.scope, &id, &data).await,
    )
    .await?;
    Ok(axum::Json(change.into()))
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
