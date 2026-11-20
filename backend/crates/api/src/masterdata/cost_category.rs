//! `/api/cost-categories`: cost categories of the account with their distribution key.

use axum::extract::{Path, State};
use axum::response::Response;
use serde::{Deserialize, Serialize};
use vermieterme_db::repo::masterdata::cost_category::{self, CostCategory, CostCategoryInput};
use vermieterme_domain::DistributionKey;

use super::fields::TEXT_MAX;
use super::{audited, audited_checked, created, deleted};
use crate::auth::AccountCtx;
use crate::error::ApiResult;
use crate::error::json::{ApiJson, timestamp};
use crate::error::validate::Validator;
use crate::routes::AppState;

/// Message for a missing or foreign category.
pub const NOT_FOUND: &str = "Kostenart nicht gefunden";
/// Range of `sortOrder`.
const SORT_RANGE: (i64, i64) = (-1_000_000, 1_000_000);

/// A category in responses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryJson {
    id: String,
    name: String,
    distribution_key: String,
    sort_order: i64,
    created_at: String,
    updated_at: String,
}

impl From<CostCategory> for CategoryJson {
    /// Response form with ISO timestamps.
    fn from(c: CostCategory) -> Self {
        Self {
            id: c.id,
            name: c.name,
            distribution_key: c.distribution_key,
            sort_order: c.sort_order,
            created_at: timestamp(c.created_at),
            updated_at: timestamp(c.updated_at),
        }
    }
}

/// Request of create and replace.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryBody {
    name: String,
    distribution_key: String,
    sort_order: Option<i64>,
}

impl CategoryBody {
    /// Validated, trimmed values; the key must be a code of [`DistributionKey`],
    /// `sortOrder` defaults to 0.
    fn validate(&self) -> ApiResult<CostCategoryInput> {
        let mut v = Validator::new();
        let codes = DistributionKey::ALL.map(DistributionKey::code);
        let input = CostCategoryInput {
            name: v.text("name", "Name", &self.name, TEXT_MAX),
            distribution_key: v.code(
                "distributionKey",
                "Verteilerschlüssel",
                &self.distribution_key,
                &codes,
            ),
            sort_order: v.int_in(
                "sortOrder",
                "Reihenfolge",
                self.sort_order.unwrap_or(0),
                SORT_RANGE,
            ),
        };
        v.finish()?;
        Ok(input)
    }
}

/// All categories by `sortOrder`.
pub async fn list(
    State(state): State<AppState>,
    ctx: AccountCtx,
) -> ApiResult<axum::Json<Vec<CategoryJson>>> {
    let mut conn = state.conn().await?;
    let items = cost_category::list(&mut conn, &ctx.scope).await?;
    Ok(axum::Json(
        items.into_iter().map(CategoryJson::from).collect(),
    ))
}

/// Creates a category.
pub async fn create(
    State(state): State<AppState>,
    ctx: AccountCtx,
    ApiJson(body): ApiJson<CategoryBody>,
) -> ApiResult<Response> {
    let input = body.validate()?;
    let category = audited(
        &state,
        &ctx,
        ("cost_category.create", "cost_category"),
        NOT_FOUND,
        |c: &CostCategory| c.id.clone(),
        async |tx| cost_category::create(tx, &ctx.scope, &input).await,
    )
    .await?;
    Ok(created(CategoryJson::from(category)))
}

/// Replaces a category.
pub async fn update(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
    ApiJson(body): ApiJson<CategoryBody>,
) -> ApiResult<axum::Json<CategoryJson>> {
    let input = body.validate()?;
    let category = audited(
        &state,
        &ctx,
        ("cost_category.update", "cost_category"),
        NOT_FOUND,
        |c: &CostCategory| c.id.clone(),
        async |tx| cost_category::update(tx, &ctx.scope, &id, &input).await,
    )
    .await?;
    Ok(axum::Json(category.into()))
}

/// Deletes a category; one that costs use is 409.
pub async fn remove(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    audited_checked(
        &state,
        &ctx,
        ("cost_category.delete", "cost_category"),
        NOT_FOUND,
        |id: &String| id.clone(),
        async |tx| {
            let outcome = cost_category::delete(tx, &ctx.scope, &id).await?;
            Ok(outcome.map(|()| id.clone()))
        },
    )
    .await?;
    Ok(deleted())
}
