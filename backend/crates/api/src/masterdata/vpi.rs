//! `/api/vpi`: the consumer price index values. They are public statistics shared by all
//! accounts: every signed-in customer reads them, only admins write (contract section 9).

use axum::extract::{Path, State};
use axum::response::Response;
use serde::{Deserialize, Serialize};
use vermieterme_db::repo::audit::{self, NewAuditEntry};
use vermieterme_db::repo::masterdata::vpi::{self, VpiRecord, VpiRow};
use vermieterme_db::{AnyConnection, DbResult, in_transaction};
use vermieterme_domain::vpi::base_year_code;

use super::{created, deleted};
use crate::auth::{AccountCtx, AdminCtx};
use crate::error::json::ApiJson;
use crate::error::validate::Validator;
use crate::error::{ApiResult, not_found_as};
use crate::routes::AppState;

/// Message for a missing value.
const NOT_FOUND: &str = "VPI-Wert nicht gefunden";
/// Plausible range of the index in tenths (index series are around 100 = 1000).
const VALUE_RANGE: (i64, i64) = (1, 100_000);

/// A value in responses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VpiJson {
    id: String,
    year: i64,
    month: i64,
    value_tenths: i64,
    base_year: i64,
}

impl From<VpiRecord> for VpiJson {
    /// Response form.
    fn from(r: VpiRecord) -> Self {
        Self {
            id: r.id,
            year: r.entry.year,
            month: r.entry.month,
            value_tenths: r.entry.value_tenths,
            base_year: r.entry.base_year,
        }
    }
}

/// Request of `POST /api/vpi`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VpiBody {
    year: i64,
    month: i64,
    value_tenths: i64,
    base_year: i64,
}

impl VpiBody {
    /// Validated values: the base year is one the retrieval supports.
    fn validate(&self) -> ApiResult<VpiRow> {
        let mut v = Validator::new();
        let row = VpiRow {
            year: v.int_in("year", "Jahr", self.year, (1990, 2200)),
            month: v.int_in("month", "Monat", self.month, (1, 12)),
            value_tenths: v.int_in("valueTenths", "Indexwert", self.value_tenths, VALUE_RANGE),
            base_year: self.base_year,
        };
        if i32::try_from(self.base_year)
            .ok()
            .and_then(base_year_code)
            .is_none()
        {
            v.fail(
                "baseYear",
                "Basisjahr wird nicht unterstützt (2010, 2015, 2020)",
            );
        }
        v.finish()?;
        Ok(row)
    }
}

/// All values, newest base year and month first.
pub async fn list(
    State(state): State<AppState>,
    _ctx: AccountCtx,
) -> ApiResult<axum::Json<Vec<VpiJson>>> {
    let mut conn = state.conn().await?;
    let rows = vpi::list(&mut conn).await?;
    Ok(axum::Json(rows.into_iter().map(VpiJson::from).collect()))
}

/// Writes the audit entry of a global change by the admin of `ctx`.
async fn record(
    conn: &mut AnyConnection,
    ctx: &AdminCtx,
    action: &'static str,
    entity_id: &str,
) -> DbResult<()> {
    let entry = NewAuditEntry {
        user_id: Some(&ctx.user.id),
        account_id: ctx.acting_account_id.as_deref(),
        action,
        entity: "vpi_entry",
        entity_id: Some(entity_id),
    };
    audit::record(conn, &entry).await.map(|_| ())
}

/// Stores a value (create or update of the same month and base year). Admin only.
pub async fn save(
    State(state): State<AppState>,
    ctx: AdminCtx,
    ApiJson(body): ApiJson<VpiBody>,
) -> ApiResult<Response> {
    let row = body.validate()?;
    let mut conn = state.conn().await?;
    let saved = in_transaction(&mut conn, async |tx| {
        let saved = vpi::upsert(tx, &ctx.scope, &row).await?;
        record(tx, &ctx, "vpi.save", &saved.id).await?;
        Ok(saved)
    })
    .await?;
    Ok(created(VpiJson::from(saved)))
}

/// Deletes a value. Admin only.
pub async fn remove(
    State(state): State<AppState>,
    ctx: AdminCtx,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let mut conn = state.conn().await?;
    in_transaction(&mut conn, async |tx| {
        vpi::delete(tx, &ctx.scope, &id).await?;
        record(tx, &ctx, "vpi.delete", &id).await
    })
    .await
    .map_err(not_found_as(NOT_FOUND))?;
    Ok(deleted())
}
