//! `/api/billing-periods/{id}/costs`: the positions of a period, saving one position
//! (`PATCH …/costs/{categoryId}`) and the amount of one unit in it
//! (`PATCH …/costs/{categoryId}/units/{unitId}`).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use vermieterme_db::repo::billing::{Cost, CostPatch, Saved, cost};
use vermieterme_domain::DistributionKey;

use super::json::CostJson;
use super::{NOT_FOUND, present};
use crate::auth::AccountCtx;
use crate::error::json::ApiJson;
use crate::error::validate::Validator;
use crate::error::{ApiResult, not_found_as};
use crate::masterdata::{audited_checked, created};
use crate::routes::AppState;

/// Message for a missing or foreign period or category.
const POSITION_NOT_FOUND: &str = "Abrechnungszeitraum oder Kostenart nicht gefunden";
/// Message for a missing or foreign period, category or unit (or a unit of another
/// property).
const UNIT_NOT_FOUND: &str = "Abrechnungszeitraum, Kostenart oder Einheit nicht gefunden";

/// Request of `PATCH …/costs/{categoryId}`: absent fields stay, `null` clears.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostBody {
    #[serde(default, deserialize_with = "present")]
    total_amount_cents: Option<Option<i64>>,
    enabled: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    distribution_key_override: Option<Option<String>>,
    reviewed: Option<bool>,
    version: Option<i64>,
}

impl CostBody {
    /// Validated changes: amount within the cent limit, override a key code (an empty
    /// text clears like `null`).
    fn validate(&self) -> ApiResult<CostPatch> {
        let mut v = Validator::new();
        if let Some(Some(total)) = self.total_amount_cents {
            v.cents("totalAmountCents", "Gesamtbetrag", total, false);
        }
        let override_key = self.distribution_key_override.as_ref().map(|code| {
            let code = code.as_deref().map(str::trim).filter(|c| !c.is_empty())?;
            let key = DistributionKey::from_code(code);
            if key.is_err() {
                v.fail("distributionKeyOverride", "Verteilerschlüssel ist ungültig");
            }
            key.ok()
        });
        v.finish()?;
        Ok(CostPatch {
            total_amount_cents: self.total_amount_cents,
            enabled: self.enabled,
            override_key,
            reviewed: self.reviewed,
            version: self.version,
        })
    }
}

/// Request of `PATCH …/units/{unitId}`: `null` clears the amount.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitAmountBody {
    #[serde(default, deserialize_with = "present")]
    amount_cents: Option<Option<i64>>,
    version: Option<i64>,
}

/// The positions of a period with their unit amounts.
pub async fn list(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
) -> ApiResult<axum::Json<Vec<CostJson>>> {
    let mut conn = state.conn().await?;
    let costs = cost::list(&mut conn, &ctx.scope, &id)
        .await
        .map_err(not_found_as(NOT_FOUND))?;
    Ok(axum::Json(costs.into_iter().map(CostJson::from).collect()))
}

/// Saves the sent fields of the position of a category (201 when created, else 200).
pub async fn save(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path((id, category_id)): Path<(String, String)>,
    ApiJson(body): ApiJson<CostBody>,
) -> ApiResult<Response> {
    let patch = body.validate()?;
    let saved = audited_checked(
        &state,
        &ctx,
        ("cost.save", "cost"),
        POSITION_NOT_FOUND,
        |s: &Saved<Cost>| s.value().id.clone(),
        async |tx| cost::save(tx, &ctx.scope, &id, &category_id, &patch).await,
    )
    .await?;
    Ok(match saved {
        Saved::Created(c) => created(CostJson::from(c)),
        Saved::Updated(c) => (StatusCode::OK, axum::Json(CostJson::from(c))).into_response(),
    })
}

/// Sets or clears the amount of one unit in a position (`per_notice` /
/// `per_attachment`, #80); the unit must belong to the period's property.
pub async fn save_unit(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path((id, category_id, unit_id)): Path<(String, String, String)>,
    ApiJson(body): ApiJson<UnitAmountBody>,
) -> ApiResult<axum::Json<CostJson>> {
    let mut v = Validator::new();
    let amount = body.amount_cents.flatten();
    if let Some(amount) = amount {
        v.cents("amountCents", "Betrag", amount, false);
    }
    if body.amount_cents.is_none() {
        v.fail("amountCents", "Betrag fehlt (null leert ihn)");
    }
    v.finish()?;
    let key = (id.as_str(), category_id.as_str(), unit_id.as_str());
    let saved = audited_checked(
        &state,
        &ctx,
        ("cost.save_unit_amount", "cost"),
        UNIT_NOT_FOUND,
        |c: &Cost| c.id.clone(),
        async |tx| cost::set_unit_amount(tx, &ctx.scope, key, amount, body.version).await,
    )
    .await?;
    Ok(axum::Json(saved.into()))
}
