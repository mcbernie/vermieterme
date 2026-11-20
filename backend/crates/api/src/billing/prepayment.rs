//! `/api/billing-periods/{id}/prepayments`: the prepayments of a period and saving the
//! prepayment of one unit (`PATCH …/prepayments/{unitId}`).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use vermieterme_db::repo::billing::{Prepayment, PrepaymentPatch, Saved, prepayment};

use super::NOT_FOUND;
use super::json::PrepaymentJson;
use crate::auth::AccountCtx;
use crate::error::json::ApiJson;
use crate::error::validate::Validator;
use crate::error::{ApiResult, not_found_as};
use crate::masterdata::audited_read_committed;
use crate::masterdata::created;
use crate::routes::AppState;

/// Message for a missing or foreign period or unit (or a unit of another property).
const UNIT_NOT_FOUND: &str = "Abrechnungszeitraum oder Einheit nicht gefunden";

/// Request of `PATCH …/prepayments/{unitId}`: absent fields stay.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepaymentBody {
    monthly_amount_cents: Option<i64>,
    reviewed: Option<bool>,
    version: Option<i64>,
}

impl PrepaymentBody {
    /// Validated changes: the monthly amount is between 0 and the cent limit.
    fn validate(&self) -> ApiResult<PrepaymentPatch> {
        let mut v = Validator::new();
        if let Some(amount) = self.monthly_amount_cents {
            v.cents("monthlyAmountCents", "Monatlicher Betrag", amount, true);
        }
        v.finish()?;
        Ok(PrepaymentPatch {
            monthly_amount_cents: self.monthly_amount_cents,
            reviewed: self.reviewed,
            version: self.version,
        })
    }
}

/// The prepayments of a period.
pub async fn list(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
) -> ApiResult<axum::Json<Vec<PrepaymentJson>>> {
    let mut conn = state.conn().await?;
    let items = prepayment::list(&mut conn, &ctx.scope, &id)
        .await
        .map_err(not_found_as(NOT_FOUND))?;
    Ok(axum::Json(
        items.into_iter().map(PrepaymentJson::from).collect(),
    ))
}

/// Saves the sent fields of the prepayment of a unit (201 when created, else 200).
pub async fn save(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path((id, unit_id)): Path<(String, String)>,
    ApiJson(body): ApiJson<PrepaymentBody>,
) -> ApiResult<Response> {
    let patch = body.validate()?;
    let saved = audited_read_committed(
        &state,
        &ctx,
        ("prepayment.save", "prepayment"),
        UNIT_NOT_FOUND,
        |s: &Saved<Prepayment>| s.value().id.clone(),
        async |tx| prepayment::save(tx, &ctx.scope, &id, &unit_id, &patch).await,
    )
    .await?;
    Ok(match saved {
        Saved::Created(p) => created(PrepaymentJson::from(p)),
        Saved::Updated(p) => (StatusCode::OK, axum::Json(PrepaymentJson::from(p))).into_response(),
    })
}
