//! `/api/tenants`: list (optionally of one unit), create, replace and delete. Tenant data
//! is personal data: values never appear in messages, logs or audit entries.

use axum::extract::{Path, State};
use axum::response::Response;
use serde::Deserialize;
use vermieterme_db::repo::masterdata::tenant::{self, LEASE_TYPES, Tenant, TenantData};
use vermieterme_db::repo::masterdata::unit as unit_repo;
use vermieterme_domain::vpi::base_year_code;

use super::fields::{ID_INPUT_MAX, SHORT_MAX, TEXT_MAX, filter};
use super::tenant_json::TenantJson;
use super::unit::NOT_FOUND as UNIT_NOT_FOUND;
use super::unit_embed;
use super::{audited, created, deleted};
use crate::auth::AccountCtx;
use crate::error::json::{ApiJson, ApiQuery};
use crate::error::validate::Validator;
use crate::error::{ApiResult, not_found_as};
use crate::routes::AppState;
use vermieterme_db::repo::masterdata::unit_ref;

/// Message for a missing or foreign tenant.
pub const NOT_FOUND: &str = "Mieter nicht gefunden";

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
            unit_id: v.text("unitId", "Einheit", &self.unit_id, ID_INPUT_MAX),
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
    let unit_id = filter(query.unit_id.as_deref());
    if let Some(unit_id) = unit_id {
        unit_repo::get(&mut conn, &ctx.scope, unit_id)
            .await
            .map_err(not_found_as(UNIT_NOT_FOUND))?;
    }
    let tenants = tenant::list(&mut conn, &ctx.scope, unit_id).await?;
    let units = unit_ref::by_id(&mut conn, &ctx.scope).await?;
    Ok(axum::Json(
        tenants
            .into_iter()
            .map(|t| TenantJson::new(t, &units))
            .collect(),
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
    let units = unit_embed::load(&state, &ctx).await?;
    Ok(created(TenantJson::new(tenant, &units)))
}

/// Tells a missing tenant from a missing target unit before the change (the repository
/// re-checks both inside its transaction), so the 404 names the right object.
async fn check_targets(
    state: &AppState,
    ctx: &AccountCtx,
    id: &str,
    unit_id: &str,
) -> ApiResult<()> {
    let mut conn = state.conn().await?;
    tenant::get(&mut conn, &ctx.scope, id)
        .await
        .map_err(not_found_as(NOT_FOUND))?;
    unit_repo::get(&mut conn, &ctx.scope, unit_id)
        .await
        .map_err(not_found_as(UNIT_NOT_FOUND))?;
    Ok(())
}

/// Replaces a tenant.
pub async fn update(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
    ApiJson(body): ApiJson<TenantBody>,
) -> ApiResult<axum::Json<TenantJson>> {
    let data = body.validate()?;
    check_targets(&state, &ctx, &id, &data.unit_id).await?;
    let tenant = audited(
        &state,
        &ctx,
        ("tenant.update", "tenant"),
        NOT_FOUND,
        |t: &Tenant| t.id.clone(),
        async |tx| tenant::update(tx, &ctx.scope, &id, &data).await,
    )
    .await?;
    let units = unit_embed::load(&state, &ctx).await?;
    Ok(axum::Json(TenantJson::new(tenant, &units)))
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
