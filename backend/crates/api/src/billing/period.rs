//! `/api/billing-periods` and `/api/billing-periods/{id}`: list, create (with copy),
//! detail, change of the dates, delete. List, create and change answer with the
//! `BillingPeriodSummary` (without totals), so legacy shares that the detail refuses
//! with 422 cannot turn a successful change into an error; the detail has the totals.

use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::response::Response;
use serde::{Deserialize, Serialize};
use vermieterme_db::repo::billing::{
    BillingPeriod, NewPeriod, PeriodPatch, PeriodSummary, Rejection, period, summary,
};
use vermieterme_db::repo::masterdata::property::{self, Property};
use vermieterme_db::{AccountScope, AnyConnection, DbError};

use super::audited::audited_read_committed;
use super::detail::{Detail, load};
use super::json::{PeriodJson, PropertyEmbedJson};
use super::{NOT_FOUND, present};
use crate::auth::AccountCtx;
use crate::error::json::{ApiJson, ApiQuery};
use crate::error::validate::Validator;
use crate::error::{ApiResult, not_found_as};
use crate::masterdata::fields::{ID_INPUT_MAX, filter};
use crate::masterdata::{created, deleted};
use crate::routes::AppState;

/// A period with its property and counts (`BillingPeriodSummary`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryJson {
    #[serde(flatten)]
    period: PeriodJson,
    property: Option<PropertyEmbedJson>,
    cost_count: usize,
    unreviewed_count: usize,
}

impl SummaryJson {
    /// Response form; `property` is `None` only if it vanished in between.
    fn new(item: PeriodSummary, property: Option<Property>) -> Self {
        Self {
            property: property.map(PropertyEmbedJson::from),
            period: item.period.into(),
            cost_count: item.cost_count,
            unreviewed_count: item.unreviewed_count,
        }
    }

    /// Loads the summary of one period (answer of create and change).
    async fn load(
        conn: &mut AnyConnection,
        scope: &AccountScope,
        found: &BillingPeriod,
    ) -> ApiResult<Self> {
        let item = summary::get(conn, scope, &found.id).await?;
        let owner = match property::get(conn, scope, &found.property_id).await {
            Ok(owner) => Some(owner),
            Err(DbError::NotFound) => None,
            Err(other) => return Err(other.into()),
        };
        Ok(Self::new(item, owner))
    }
}

/// Query of the list.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListQuery {
    property_id: Option<String>,
}

/// Request of create.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBody {
    property_id: String,
    start_date: String,
    end_date: String,
    billing_date: Option<String>,
    copy_from_id: Option<String>,
}

impl CreateBody {
    /// Validated values: dates parse, the end lies after the start.
    fn validate(&self) -> ApiResult<NewPeriod> {
        let mut v = Validator::new();
        let input = NewPeriod {
            property_id: v.text("propertyId", "Objekt", &self.property_id, ID_INPUT_MAX),
            start_date: v.date("startDate", "Beginn", &self.start_date),
            end_date: v.date("endDate", "Ende", &self.end_date),
            billing_date: v.opt_date(
                "billingDate",
                "Abrechnungsdatum",
                self.billing_date.as_deref(),
            ),
            copy_from_id: v.opt_text(
                "copyFromId",
                "Vorlage",
                self.copy_from_id.as_deref(),
                ID_INPUT_MAX,
            ),
        };
        if !input.start_date.is_empty() && input.end_date <= input.start_date {
            v.fail("endDate", "Das Enddatum muss nach dem Beginndatum liegen");
        }
        v.finish()?;
        Ok(input)
    }
}

/// Request of the change: absent fields stay, `null` clears an optional date.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBody {
    #[serde(default, deserialize_with = "present")]
    start_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "present")]
    end_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "present")]
    billing_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "present")]
    sent_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "present")]
    paid_date: Option<Option<String>>,
}

impl UpdateBody {
    /// Validated changes; begin and end cannot be cleared (an empty text clears like
    /// `null` for the optional dates).
    fn validate(&self) -> ApiResult<PeriodPatch> {
        let mut v = Validator::new();
        let mut required = |field: &str, label: &str, value: &Option<Option<String>>| {
            value
                .as_ref()
                .map(|value| v.date(field, label, value.as_deref().unwrap_or_default()))
        };
        let start_date = required("startDate", "Beginn", &self.start_date);
        let end_date = required("endDate", "Ende", &self.end_date);
        let mut optional = |field: &str, label: &str, value: &Option<Option<String>>| {
            value
                .as_ref()
                .map(|value| v.opt_date(field, label, value.as_deref()))
        };
        let patch = PeriodPatch {
            start_date,
            end_date,
            billing_date: optional("billingDate", "Abrechnungsdatum", &self.billing_date),
            sent_date: optional("sentDate", "Versanddatum", &self.sent_date),
            paid_date: optional("paidDate", "Zahlungsdatum", &self.paid_date),
        };
        v.finish()?;
        Ok(patch)
    }
}

/// Periods of the account, latest first, optionally of one property; each with its
/// property, the number of cost positions and of open reviews.
pub async fn list(
    State(state): State<AppState>,
    ctx: AccountCtx,
    ApiQuery(query): ApiQuery<ListQuery>,
) -> ApiResult<axum::Json<Vec<SummaryJson>>> {
    let mut conn = state.conn().await?;
    let property_id = filter(query.property_id.as_deref());
    let items = summary::list(&mut conn, &ctx.scope, property_id).await?;
    let properties: HashMap<String, Property> = property::list(&mut conn, &ctx.scope)
        .await?
        .into_iter()
        .map(|s| (s.property.id.clone(), s.property))
        .collect();
    let out = items
        .into_iter()
        .map(|item| {
            let owner = properties.get(&item.period.property_id).cloned();
            SummaryJson::new(item, owner)
        })
        .collect();
    Ok(axum::Json(out))
}

/// Creates a period (201 with the summary); with `copyFromId` the positions of that
/// period of the same property are copied, unreviewed.
pub async fn create(
    State(state): State<AppState>,
    ctx: AccountCtx,
    ApiJson(body): ApiJson<CreateBody>,
) -> ApiResult<Response> {
    let input = body.validate()?;
    let made = audited_read_committed(
        &state,
        &ctx,
        ("billing_period.create", "billing_period"),
        crate::masterdata::property::NOT_FOUND,
        |p: &BillingPeriod| p.id.clone(),
        async |tx| period::create(tx, &ctx.scope, &input).await,
    )
    .await?;
    let mut conn = state.conn().await?;
    Ok(created(
        SummaryJson::load(&mut conn, &ctx.scope, &made).await?,
    ))
}

/// One period with property, units, tenants, positions and the totals per unit.
pub async fn detail(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
) -> ApiResult<axum::Json<Detail>> {
    let mut conn = state.conn().await?;
    let found = period::get(&mut conn, &ctx.scope, &id)
        .await
        .map_err(not_found_as(NOT_FOUND))?;
    load(&mut conn, &ctx.scope, found).await.map(axum::Json)
}

/// Changes the dates of a period (200 with the summary); the property cannot change.
pub async fn update(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
    ApiJson(body): ApiJson<UpdateBody>,
) -> ApiResult<axum::Json<SummaryJson>> {
    let patch = body.validate()?;
    let changed = audited_read_committed(
        &state,
        &ctx,
        ("billing_period.update", "billing_period"),
        NOT_FOUND,
        |p: &BillingPeriod| p.id.clone(),
        async |tx| period::update(tx, &ctx.scope, &id, &patch).await,
    )
    .await?;
    let mut conn = state.conn().await?;
    SummaryJson::load(&mut conn, &ctx.scope, &changed)
        .await
        .map(axum::Json)
}

/// Deletes a period with its positions and documents.
pub async fn remove(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    audited_read_committed(
        &state,
        &ctx,
        ("billing_period.delete", "billing_period"),
        NOT_FOUND,
        |id: &String| id.clone(),
        async |tx| {
            period::delete(tx, &ctx.scope, &id).await?;
            Ok(Ok::<_, Rejection>(id.clone()))
        },
    )
    .await?;
    Ok(deleted())
}
