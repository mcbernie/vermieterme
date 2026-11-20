//! `/api/tenants`: list (optionally of one unit), create, replace and delete. Tenant data
//! is personal data: values never appear in messages, logs or audit entries.

use axum::extract::{Path, State};
use axum::response::Response;
use serde::{Deserialize, Serialize};
use vermieterme_db::repo::masterdata::tenant::{self, LEASE_TYPES, Tenant, TenantData};
use vermieterme_db::repo::masterdata::unit as unit_repo;
use vermieterme_domain::vpi::base_year_code;

use super::fields::{SHORT_MAX, TEXT_MAX};
use super::unit::NOT_FOUND as UNIT_NOT_FOUND;
use super::{audited, created, deleted};
use crate::auth::AccountCtx;
use crate::error::json::{ApiJson, ApiQuery, timestamp};
use crate::error::validate::Validator;
use crate::error::{ApiResult, not_found_as};
use crate::routes::AppState;

/// Message for a missing or foreign tenant.
pub const NOT_FOUND: &str = "Mieter nicht gefunden";

/// A tenant in responses.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantJson {
    id: String,
    unit_id: String,
    salutation: String,
    first_name: String,
    last_name: String,
    salutation2: Option<String>,
    first_name2: Option<String>,
    last_name2: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    bank_name: Option<String>,
    iban: Option<String>,
    account_holder: Option<String>,
    move_in_date: String,
    move_out_date: Option<String>,
    lease_type: String,
    index_base_year: Option<i64>,
    index_reference_value_tenths: Option<i64>,
    index_reference_date: Option<String>,
    index_min_months: i64,
    created_at: String,
    updated_at: String,
}

impl From<Tenant> for TenantJson {
    /// Response form with ISO timestamps.
    fn from(t: Tenant) -> Self {
        let d = t.data;
        Self {
            id: t.id,
            unit_id: d.unit_id,
            salutation: d.salutation,
            first_name: d.first_name,
            last_name: d.last_name,
            salutation2: d.salutation2,
            first_name2: d.first_name2,
            last_name2: d.last_name2,
            phone: d.phone,
            email: d.email,
            bank_name: d.bank_name,
            iban: d.iban,
            account_holder: d.account_holder,
            move_in_date: d.move_in_date,
            move_out_date: d.move_out_date,
            lease_type: d.lease_type,
            index_base_year: d.index_base_year,
            index_reference_value_tenths: d.index_reference_tenths,
            index_reference_date: d.index_reference_date,
            index_min_months: d.index_min_months,
            created_at: timestamp(t.created_at),
            updated_at: timestamp(t.updated_at),
        }
    }
}

/// Query of the list.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantQuery {
    unit_id: Option<String>,
}

/// Request of create and replace.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantBody {
    unit_id: String,
    salutation: String,
    first_name: String,
    last_name: String,
    salutation2: Option<String>,
    first_name2: Option<String>,
    last_name2: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    bank_name: Option<String>,
    iban: Option<String>,
    account_holder: Option<String>,
    move_in_date: String,
    move_out_date: Option<String>,
    lease_type: Option<String>,
    index_base_year: Option<i64>,
    index_reference_value_tenths: Option<i64>,
    index_reference_date: Option<String>,
    index_min_months: Option<i64>,
}

impl TenantBody {
    /// Validated, trimmed values. Empty optional text becomes `null`; the move-out date
    /// must not lie before the move-in date; `leaseType` defaults to `standard` and
    /// `indexMinMonths` to 12.
    fn validate(&self) -> ApiResult<TenantData> {
        let mut v = Validator::new();
        let move_in = v.date("moveInDate", "Einzugsdatum", &self.move_in_date);
        let move_out = v.opt_date("moveOutDate", "Auszugsdatum", self.move_out_date.as_deref());
        // ISO dates compare chronologically as text.
        if move_out
            .as_deref()
            .is_some_and(|out| out < move_in.as_str())
            && !move_in.is_empty()
        {
            v.fail("moveOutDate", "Auszugsdatum liegt vor dem Einzugsdatum");
        }
        let data = TenantData {
            unit_id: v.text("unitId", "Einheit", &self.unit_id, 36),
            salutation: v.text("salutation", "Anrede", &self.salutation, SHORT_MAX),
            first_name: v.text("firstName", "Vorname", &self.first_name, TEXT_MAX),
            last_name: v.text("lastName", "Nachname", &self.last_name, TEXT_MAX),
            salutation2: v.opt_text(
                "salutation2",
                "Anrede",
                self.salutation2.as_deref(),
                SHORT_MAX,
            ),
            first_name2: v.opt_text(
                "firstName2",
                "Vorname",
                self.first_name2.as_deref(),
                TEXT_MAX,
            ),
            last_name2: v.opt_text(
                "lastName2",
                "Nachname",
                self.last_name2.as_deref(),
                TEXT_MAX,
            ),
            phone: v.opt_text("phone", "Telefon", self.phone.as_deref(), SHORT_MAX),
            email: v.opt_email("email", self.email.as_deref()),
            bank_name: v.opt_text("bankName", "Bank", self.bank_name.as_deref(), TEXT_MAX),
            iban: v.opt_iban("iban", self.iban.as_deref()),
            account_holder: v.opt_text(
                "accountHolder",
                "Kontoinhaber",
                self.account_holder.as_deref(),
                TEXT_MAX,
            ),
            move_in_date: move_in,
            move_out_date: move_out,
            lease_type: v.code(
                "leaseType",
                "Mietart",
                self.lease_type.as_deref().unwrap_or("standard"),
                &LEASE_TYPES,
            ),
            index_base_year: self.index_base_year,
            index_reference_tenths: self.index_reference_value_tenths,
            index_reference_date: v.opt_date(
                "indexReferenceDate",
                "Datum der letzten Anpassung",
                self.index_reference_date.as_deref(),
            ),
            index_min_months: v.int_in(
                "indexMinMonths",
                "Mindestabstand",
                self.index_min_months.unwrap_or(12),
                (0, 240),
            ),
        };
        if let Some(year) = data.index_base_year
            && i32::try_from(year).ok().and_then(base_year_code).is_none()
        {
            v.fail(
                "indexBaseYear",
                "Basisjahr wird nicht unterstützt (2010, 2015, 2020)",
            );
        }
        if let Some(value) = data.index_reference_tenths {
            v.int_in(
                "indexReferenceValueTenths",
                "Indexwert",
                value,
                (1, 1_000_000),
            );
        }
        v.finish()?;
        Ok(data)
    }
}

/// Tenants of the account, optionally of one unit (a foreign or unknown unit is 404).
pub async fn list(
    State(state): State<AppState>,
    ctx: AccountCtx,
    ApiQuery(query): ApiQuery<TenantQuery>,
) -> ApiResult<axum::Json<Vec<TenantJson>>> {
    let mut conn = state.conn().await?;
    if let Some(unit_id) = query.unit_id.as_deref() {
        unit_repo::get(&mut conn, &ctx.scope, unit_id)
            .await
            .map_err(not_found_as(UNIT_NOT_FOUND))?;
    }
    let tenants = tenant::list(&mut conn, &ctx.scope, query.unit_id.as_deref()).await?;
    Ok(axum::Json(
        tenants.into_iter().map(TenantJson::from).collect(),
    ))
}

/// Creates a tenant.
pub async fn create(
    State(state): State<AppState>,
    ctx: AccountCtx,
    ApiJson(body): ApiJson<TenantBody>,
) -> ApiResult<Response> {
    let data = body.validate()?;
    let tenant = audited(
        &state,
        &ctx,
        ("tenant.create", "tenant"),
        UNIT_NOT_FOUND,
        |t: &Tenant| t.id.clone(),
        async |tx| tenant::create(tx, &ctx.scope, &data).await,
    )
    .await?;
    Ok(created(TenantJson::from(tenant)))
}

/// Replaces a tenant.
pub async fn update(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
    ApiJson(body): ApiJson<TenantBody>,
) -> ApiResult<axum::Json<TenantJson>> {
    let data = body.validate()?;
    let tenant = audited(
        &state,
        &ctx,
        ("tenant.update", "tenant"),
        NOT_FOUND,
        |t: &Tenant| t.id.clone(),
        async |tx| tenant::update(tx, &ctx.scope, &id, &data).await,
    )
    .await?;
    Ok(axum::Json(tenant.into()))
}

/// Deletes a tenant.
pub async fn remove(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    audited(
        &state,
        &ctx,
        ("tenant.delete", "tenant"),
        NOT_FOUND,
        |id: &String| id.clone(),
        async |tx| {
            tenant::delete(tx, &ctx.scope, &id).await?;
            Ok(id.clone())
        },
    )
    .await?;
    Ok(deleted())
}
