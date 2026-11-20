//! `/api/properties`: list with unit counts, detail with units and tenants, create,
//! replace and delete.

use axum::extract::{Path, State};
use axum::response::Response;
use serde::{Deserialize, Serialize};
use vermieterme_db::repo::documents::Owner;
use vermieterme_db::repo::masterdata::property::{self, Property, PropertyInput};
use vermieterme_db::repo::masterdata::tenant;
use vermieterme_db::repo::masterdata::unit::{self, Unit};
use vermieterme_db::{AccountScope, AnyConnection};

use super::fields::{TEXT_MAX, ZIP_MAX};
use super::tenant_json::TenantJson;
use super::unit::UnitJson;
use super::{audited, created, deleted};
use crate::auth::AccountCtx;
use crate::documents::cascade::delete_owner;
use crate::error::json::{ApiJson, timestamp};
use crate::error::validate::Validator;
use crate::error::{ApiResult, not_found_as};
use crate::routes::AppState;

/// Message for a missing or foreign property.
pub const NOT_FOUND: &str = "Objekt nicht gefunden";
/// Upper bound of shares (keeps products of shares and cents far from overflow).
pub const SHARES_MAX: i64 = 1_000_000_000;

/// A property in responses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyJson {
    id: String,
    street: String,
    zip: String,
    city: String,
    total_shares: i64,
    created_at: String,
    updated_at: String,
}

impl From<Property> for PropertyJson {
    /// Response form with ISO timestamps.
    fn from(p: Property) -> Self {
        Self {
            id: p.id,
            street: p.street,
            zip: p.zip,
            city: p.city,
            total_shares: p.total_shares,
            created_at: timestamp(p.created_at),
            updated_at: timestamp(p.updated_at),
        }
    }
}

/// A property in the list.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyListItem {
    #[serde(flatten)]
    property: PropertyJson,
    unit_count: i64,
}

/// A unit with its tenants (inside the property detail).
#[derive(Debug, Serialize)]
pub struct UnitWithTenants {
    #[serde(flatten)]
    unit: UnitJson,
    tenants: Vec<TenantJson>,
}

/// The property detail.
#[derive(Debug, Serialize)]
pub struct PropertyDetail {
    #[serde(flatten)]
    property: PropertyJson,
    units: Vec<UnitWithTenants>,
}

/// Request of create and replace.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyBody {
    street: String,
    zip: String,
    city: String,
    total_shares: i64,
}

impl PropertyBody {
    /// Validated, trimmed values.
    fn validate(&self) -> ApiResult<PropertyInput> {
        let mut v = Validator::new();
        let input = PropertyInput {
            street: v.text("street", "Straße", &self.street, TEXT_MAX),
            zip: v.text("zip", "Postleitzahl", &self.zip, ZIP_MAX),
            city: v.text("city", "Ort", &self.city, TEXT_MAX),
            total_shares: v.int_in(
                "totalShares",
                "Gesamtanteile",
                self.total_shares,
                (1, SHARES_MAX),
            ),
        };
        v.finish()?;
        Ok(input)
    }
}

/// All properties with their unit counts.
pub async fn list(
    State(state): State<AppState>,
    ctx: AccountCtx,
) -> ApiResult<axum::Json<Vec<PropertyListItem>>> {
    let mut conn = state.conn().await?;
    let items = property::list(&mut conn, &ctx.scope).await?;
    Ok(axum::Json(
        items
            .into_iter()
            .map(|s| PropertyListItem {
                property: s.property.into(),
                unit_count: s.unit_count,
            })
            .collect(),
    ))
}

/// One property with its units and their tenants.
pub async fn detail(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
) -> ApiResult<axum::Json<PropertyDetail>> {
    let mut conn = state.conn().await?;
    load_detail(&mut conn, &ctx.scope, &id)
        .await
        .map(|loaded| axum::Json(loaded.detail))
}

/// A property detail together with the rows it was built from, so the billing detail
/// computes its totals without reading them again (#115).
pub(crate) struct LoadedDetail {
    /// The response form.
    pub detail: PropertyDetail,
    /// The property row.
    pub property: Property,
    /// Its units, newest first.
    pub units: Vec<Unit>,
}

/// Loads a property with its units and their tenants (also embedded in the billing
/// detail). A missing or foreign property is 404.
pub(crate) async fn load_detail(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
) -> ApiResult<LoadedDetail> {
    let found = property::get(conn, scope, id)
        .await
        .map_err(not_found_as(NOT_FOUND))?;
    let units = unit::list(conn, scope, Some(id)).await?;
    let tenants = tenant::list_for_property(conn, scope, id).await?;
    // The tenants are nested in their unit here, so the embedded `unit` is left out.
    let no_units = std::collections::HashMap::new();
    let nested = units
        .iter()
        .cloned()
        .map(|u| UnitWithTenants {
            tenants: tenants
                .iter()
                .filter(|t| t.data.unit_id == u.id)
                .cloned()
                .map(|t| TenantJson::new(t, &no_units))
                .collect(),
            unit: u.into(),
        })
        .collect();
    Ok(LoadedDetail {
        detail: PropertyDetail {
            property: found.clone().into(),
            units: nested,
        },
        property: found,
        units,
    })
}

/// Creates a property.
pub async fn create(
    State(state): State<AppState>,
    ctx: AccountCtx,
    ApiJson(body): ApiJson<PropertyBody>,
) -> ApiResult<Response> {
    let input = body.validate()?;
    let property = audited(
        &state,
        &ctx,
        ("property.create", "property"),
        NOT_FOUND,
        |p: &Property| p.id.clone(),
        async |tx| property::create(tx, &ctx.scope, &input).await,
    )
    .await?;
    Ok(created(PropertyJson::from(property)))
}

/// Replaces a property; total shares below a unit's shares are refused.
pub async fn update(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
    ApiJson(body): ApiJson<PropertyBody>,
) -> ApiResult<axum::Json<PropertyJson>> {
    let input = body.validate()?;
    let property = super::audited_checked(
        &state,
        &ctx,
        ("property.update", "property"),
        NOT_FOUND,
        |p: &Property| p.id.clone(),
        async |tx| property::update(tx, &ctx.scope, &id, &input).await,
    )
    .await?;
    Ok(axum::Json(property.into()))
}

/// Deletes a property with everything below it.
pub async fn remove(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    // The cascade reaches units, billing tables and documents: same isolation as billing
    // (#116); the files of the documents go after the commit (#106).
    delete_owner(
        &state,
        &ctx,
        ("property.delete", "property"),
        NOT_FOUND,
        Owner::Property(&id),
        async |tx| property::delete(tx, &ctx.scope, &id).await,
    )
    .await?;
    Ok(deleted())
}
