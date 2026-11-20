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
use vermieterme_db::repo::documents::Owner;
use vermieterme_db::repo::masterdata::property::{self, Property};
use vermieterme_db::{AccountScope, AnyConnection, DbError, DbResult};

use super::detail::{Detail, load};
use super::json::PeriodJson;
use super::{NOT_FOUND, present};
use crate::auth::AccountCtx;
use crate::documents::cascade::delete_owner;
use crate::error::json::{ApiJson, ApiQuery};
use crate::error::validate::Validator;
use crate::error::{ApiResult, not_found_as};
use crate::masterdata::audited_read_committed;
use crate::masterdata::fields::{ID_INPUT_MAX, filter};
use crate::masterdata::unit_embed::PropertyEmbed;
use crate::masterdata::{created, deleted};
use crate::routes::AppState;

/// A period with its property and counts (`BillingPeriodSummary`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryJson {
    #[serde(flatten)]
    period: PeriodJson,
    property: Option<PropertyEmbed>,
    cost_count: usize,
    unreviewed_count: usize,
}

impl SummaryJson {
    /// Response form; `property` is `None` only if it vanished in between.
    fn new(item: PeriodSummary, property: Option<Property>) -> Self {
        Self {
            property: property.map(PropertyEmbed::from),
            period: item.period.into(),
            cost_count: item.cost_count,
            unreviewed_count: item.unreviewed_count,
        }
    }

    /// Loads the summary of one period (answer of create and change), inside the
    /// transaction of the change so a parallel delete cannot turn it into a 404.
    async fn load(
        conn: &mut AnyConnection,
        scope: &AccountScope,
        found: &BillingPeriod,
    ) -> DbResult<Self> {
        let item = summary::get(conn, scope, &found.id).await?;
        let owner = optional(property::get(conn, scope, &found.property_id).await)?;
        Ok(Self::new(item, owner))
    }
}

/// `Ok(None)` for a missing row, other errors unchanged.
fn optional<T>(found: DbResult<T>) -> DbResult<Option<T>> {
    match found {
        Ok(value) => Ok(Some(value)),
        Err(DbError::NotFound) => Ok(None),
        Err(other) => Err(other),
    }
}

/// The properties to embed in the list by id: only the filtered one if a filter is set
/// (none if it is missing or foreign), otherwise all of the account.
async fn owners(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    property_id: Option<&str>,
) -> DbResult<HashMap<String, Property>> {
    let found = match property_id {
        Some(id) => optional(property::get(conn, scope, id).await)?
            .into_iter()
            .collect(),
        None => property::list(conn, scope)
            .await?
            .into_iter()
            .map(|s| s.property)
            .collect::<Vec<_>>(),
    };
    Ok(found.into_iter().map(|p| (p.id.clone(), p)).collect())
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
    let properties = owners(&mut conn, &ctx.scope, property_id).await?;
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
    let (_, made) = audited_read_committed(
        &state,
        &ctx,
        ("billing_period.create", "billing_period"),
        crate::masterdata::property::NOT_FOUND,
        |(id, _): &(String, SummaryJson)| id.clone(),
        async |tx| {
            let made = period::create(tx, &ctx.scope, &input).await?;
            with_summary(tx, &ctx.scope, made).await
        },
    )
    .await?;
    Ok(created(made))
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
    let (_, changed) = audited_read_committed(
        &state,
        &ctx,
        ("billing_period.update", "billing_period"),
        NOT_FOUND,
        |(id, _): &(String, SummaryJson)| id.clone(),
        async |tx| {
            let changed = period::update(tx, &ctx.scope, &id, &patch).await?;
            with_summary(tx, &ctx.scope, changed).await
        },
    )
    .await?;
    Ok(axum::Json(changed))
}

/// The id and summary of a period just written, loaded in the same transaction; a
/// refusal passes through unchanged.
async fn with_summary(
    tx: &mut AnyConnection,
    scope: &AccountScope,
    written: Result<BillingPeriod, Rejection>,
) -> DbResult<Result<(String, SummaryJson), Rejection>> {
    let found = match written {
        Ok(found) => found,
        Err(rejection) => return Ok(Err(rejection)),
    };
    let summary = SummaryJson::load(tx, scope, &found).await?;
    Ok(Ok((found.id, summary)))
}

/// Deletes a period with its positions and documents.
pub async fn remove(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    delete_owner(
        &state,
        &ctx,
        ("billing_period.delete", "billing_period"),
        NOT_FOUND,
        Owner::BillingPeriod(&id),
    )
    .await?;
    Ok(deleted())
}
