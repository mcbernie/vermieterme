//! `/api/units`: list (optionally of one property), create, replace and delete.

use axum::extract::{Path, State};
use axum::response::Response;
use serde::{Deserialize, Serialize};
use vermieterme_db::repo::masterdata::property as property_repo;
use vermieterme_db::repo::masterdata::unit::{self, KINDS, Unit, UnitInput};

use super::fields::{ID_INPUT_MAX, TEXT_MAX, filter};
use super::property::{NOT_FOUND as PROPERTY_NOT_FOUND, SHARES_MAX};
use super::{audited, audited_checked, created, deleted};
use crate::auth::AccountCtx;
use crate::error::json::{ApiJson, ApiQuery, timestamp};
use crate::error::validate::Validator;
use crate::error::{ApiResult, not_found_as};
use crate::routes::AppState;

/// Message for a missing or foreign unit.
pub const NOT_FOUND: &str = "Einheit nicht gefunden";

/// A unit in responses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitJson {
    id: String,
    property_id: String,
    name: String,
    floor: String,
    shares: i64,
    kind: String,
    created_at: String,
    updated_at: String,
}

impl From<Unit> for UnitJson {
    /// Response form with ISO timestamps.
    fn from(u: Unit) -> Self {
        Self {
            id: u.id,
            property_id: u.property_id,
            name: u.name,
            floor: u.floor,
            shares: u.shares,
            kind: u.kind,
            created_at: timestamp(u.created_at),
            updated_at: timestamp(u.updated_at),
        }
    }
}

/// Query of the list.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitQuery {
    property_id: Option<String>,
}

/// Request of create and replace.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitBody {
    property_id: String,
    name: String,
    #[serde(default)]
    floor: String,
    shares: i64,
    kind: Option<String>,
}

impl UnitBody {
    /// Validated, trimmed values; `kind` defaults to `apartment`. The check against the
    /// property's total shares needs the database and happens in the repository.
    fn validate(&self) -> ApiResult<UnitInput> {
        let mut v = Validator::new();
        let kind = self.kind.as_deref().unwrap_or("apartment");
        let input = UnitInput {
            property_id: v.text("propertyId", "Objekt", &self.property_id, ID_INPUT_MAX),
            name: v.text("name", "Name", &self.name, TEXT_MAX),
            floor: v
                .opt_text("floor", "Etage", Some(&self.floor), TEXT_MAX)
                .unwrap_or_default(),
            shares: v.int_in("shares", "Anteile", self.shares, (0, SHARES_MAX)),
            kind: v.code("kind", "Art", kind, &KINDS),
        };
        v.finish()?;
        Ok(input)
    }
}

/// Units of the account, optionally of one property (a foreign or unknown property is 404).
pub async fn list(
    State(state): State<AppState>,
    ctx: AccountCtx,
    ApiQuery(query): ApiQuery<UnitQuery>,
) -> ApiResult<axum::Json<Vec<UnitJson>>> {
    let mut conn = state.conn().await?;
    let property_id = filter(query.property_id.as_deref());
    if let Some(property_id) = property_id {
        property_repo::get(&mut conn, &ctx.scope, property_id)
            .await
            .map_err(not_found_as(PROPERTY_NOT_FOUND))?;
    }
    let units = unit::list(&mut conn, &ctx.scope, property_id).await?;
    Ok(axum::Json(units.into_iter().map(UnitJson::from).collect()))
}

/// Creates a unit; shares above the property's total are 400.
pub async fn create(
    State(state): State<AppState>,
    ctx: AccountCtx,
    ApiJson(body): ApiJson<UnitBody>,
) -> ApiResult<Response> {
    let input = body.validate()?;
    let unit = audited_checked(
        &state,
        &ctx,
        ("unit.create", "unit"),
        PROPERTY_NOT_FOUND,
        |u: &Unit| u.id.clone(),
        async |tx| unit::create(tx, &ctx.scope, &input).await,
    )
    .await?;
    Ok(created(UnitJson::from(unit)))
}

/// Tells a missing unit from a missing target property before the change (the repository
/// re-checks both inside its transaction), so the 404 names the right object.
async fn check_targets(
    state: &AppState,
    ctx: &AccountCtx,
    id: &str,
    property_id: &str,
) -> ApiResult<()> {
    let mut conn = state.conn().await?;
    unit::get(&mut conn, &ctx.scope, id)
        .await
        .map_err(not_found_as(NOT_FOUND))?;
    property_repo::get(&mut conn, &ctx.scope, property_id)
        .await
        .map_err(not_found_as(PROPERTY_NOT_FOUND))?;
    Ok(())
}

/// Replaces a unit.
pub async fn update(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
    ApiJson(body): ApiJson<UnitBody>,
) -> ApiResult<axum::Json<UnitJson>> {
    let input = body.validate()?;
    check_targets(&state, &ctx, &id, &input.property_id).await?;
    let unit = audited_checked(
        &state,
        &ctx,
        ("unit.update", "unit"),
        NOT_FOUND,
        |u: &Unit| u.id.clone(),
        async |tx| unit::update(tx, &ctx.scope, &id, &input).await,
    )
    .await?;
    Ok(axum::Json(unit.into()))
}

/// Deletes a unit with its tenants and billing data.
pub async fn remove(
    State(state): State<AppState>,
    ctx: AccountCtx,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    audited(
        &state,
        &ctx,
        ("unit.delete", "unit"),
        NOT_FOUND,
        |id: &String| id.clone(),
        async |tx| {
            unit::delete(tx, &ctx.scope, &id).await?;
            Ok(id.clone())
        },
    )
    .await?;
    Ok(deleted())
}
