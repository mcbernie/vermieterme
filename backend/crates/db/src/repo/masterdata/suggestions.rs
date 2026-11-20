//! Input data of the index rent proposals: active index tenants with their latest rent.
//! The calculation itself lives in `vermieterme_domain::vpi::index_adjustment`.

use sqlx::{AnyConnection, Row};

use crate::error::DbResult;
use crate::scope::AccountScope;

/// An active tenant with an index lease and what is needed to propose a new rent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexTenant {
    /// Tenant id.
    pub tenant_id: String,
    /// Unit id.
    pub unit_id: String,
    /// First name.
    pub first_name: String,
    /// Last name.
    pub last_name: String,
    /// Unit name.
    pub unit_name: String,
    /// Street of the property.
    pub street: String,
    /// City of the property.
    pub city: String,
    /// Base year of the tenant's index series.
    pub base_year: i64,
    /// VPI at the last adjustment in tenths.
    pub reference_tenths: i64,
    /// Date of the last adjustment (`YYYY-MM-DD`), if recorded.
    pub reference_date: Option<String>,
    /// Move-in date (`YYYY-MM-DD`), the reference without an adjustment date.
    pub move_in_date: String,
    /// Minimum months between adjustments.
    pub min_months: i64,
    /// Latest rent of the unit in cents (change of type `rent`), `None` without one.
    pub current_rent_cents: Option<i64>,
}

/// Active index tenants (no move-out date, base year and reference value set) of the
/// account with the latest rent of their unit.
pub async fn index_tenants(
    conn: &mut AnyConnection,
    scope: &AccountScope,
) -> DbResult<Vec<IndexTenant>> {
    let rows = sqlx::query(
        "SELECT t.id, t.unit_id, t.first_name, t.last_name, u.name AS unit_name, p.street, \
         p.city, t.index_base_year, t.index_reference_tenths, t.index_reference_date, \
         t.move_in_date, t.index_min_months, \
         (SELECT r.amount_cents FROM rent_change r WHERE r.account_id = t.account_id \
          AND r.unit_id = t.unit_id AND r.change_type = 'rent' \
          ORDER BY r.effective_date DESC, r.created_at DESC, r.id DESC LIMIT 1) AS rent_cents \
         FROM tenant t \
         JOIN unit u ON u.account_id = t.account_id AND u.id = t.unit_id \
         JOIN property p ON p.account_id = u.account_id AND p.id = u.property_id \
         WHERE t.account_id = ? AND t.lease_type = 'index' AND t.move_out_date IS NULL \
         AND t.index_base_year IS NOT NULL AND t.index_reference_tenths IS NOT NULL \
         ORDER BY t.last_name, t.first_name, t.id",
    )
    .bind(scope.account_id())
    .fetch_all(&mut *conn)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(IndexTenant {
                tenant_id: row.try_get("id")?,
                unit_id: row.try_get("unit_id")?,
                first_name: row.try_get("first_name")?,
                last_name: row.try_get("last_name")?,
                unit_name: row.try_get("unit_name")?,
                street: row.try_get("street")?,
                city: row.try_get("city")?,
                base_year: row.try_get("index_base_year")?,
                reference_tenths: row.try_get("index_reference_tenths")?,
                reference_date: row.try_get("index_reference_date")?,
                move_in_date: row.try_get("move_in_date")?,
                min_months: row.try_get("index_min_months")?,
                current_rent_cents: row.try_get("rent_cents")?,
            })
        })
        .collect()
}
