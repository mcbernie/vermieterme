//! The detail of a billing period with the totals per unit. All sums come from the
//! domain ([`unit_statement`]); this module only gathers its inputs.

use serde::Serialize;
use vermieterme_db::repo::billing::{BillingPeriod, Cost, Prepayment, cost, prepayment};
use vermieterme_db::repo::masterdata::property::Property;
use vermieterme_db::repo::masterdata::unit::Unit;
use vermieterme_db::{AccountScope, AnyConnection, DbError};
use vermieterme_domain::billing::{CostPosition, Period, UnitBasis, unit_statement};
use vermieterme_domain::months::DEFAULT_MONTH_RULE;
use vermieterme_domain::{CalendarDate, Cents};

use super::json::{CostJson, PeriodJson, PrepaymentJson};
use crate::error::{ApiError, ApiResult};
use crate::masterdata::property::{PropertyDetail, load_detail};

/// Result of one unit (`totalsByUnit` of the contract).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitTotalJson {
    unit_id: String,
    total_unit_costs_cents: i64,
    total_prepayment_cents: i64,
    /// Prepayment minus costs: positive = refund, negative = back payment.
    difference_cents: i64,
    months: u32,
}

/// The detail of a period.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    #[serde(flatten)]
    period: PeriodJson,
    property: PropertyDetail,
    costs: Vec<CostJson>,
    prepayments: Vec<PrepaymentJson>,
    totals_by_unit: Vec<UnitTotalJson>,
}

/// Loads everything around `found` and computes the totals. Inconsistent shares of a
/// unit with an active MEA position are 422 `invalid_shares` naming the unit (#85).
pub async fn load(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    found: BillingPeriod,
) -> ApiResult<Detail> {
    let loaded = load_detail(conn, scope, &found.property_id).await?;
    let costs = cost::list(conn, scope, &found.id).await?;
    let prepayments = prepayment::list(conn, scope, &found.id).await?;
    let totals_by_unit = totals(
        &found,
        &loaded.property,
        &loaded.units,
        &costs,
        &prepayments,
    )?;
    Ok(Detail {
        period: found.into(),
        property: loaded.detail,
        costs: costs.into_iter().map(CostJson::from).collect(),
        prepayments: prepayments.into_iter().map(PrepaymentJson::from).collect(),
        totals_by_unit,
    })
}

/// A stored calendar date; unreadable dates are corrupt data (500).
fn stored_date(text: &str, column: &'static str) -> ApiResult<CalendarDate> {
    CalendarDate::parse(text).map_err(|_| ApiError::from(DbError::InvalidValue(column)))
}

/// The statement of every unit of the property, through the domain.
fn totals(
    found: &BillingPeriod,
    owner: &Property,
    units: &[Unit],
    costs: &[Cost],
    prepayments: &[Prepayment],
) -> ApiResult<Vec<UnitTotalJson>> {
    let range = Period {
        start: stored_date(&found.start_date, "start_date")?,
        end: stored_date(&found.end_date, "end_date")?,
    };
    let unit_count = i64::try_from(units.len()).unwrap_or(i64::MAX);
    units
        .iter()
        .map(|u| {
            let positions: Vec<CostPosition> = costs
                .iter()
                .map(|c| CostPosition {
                    total_amount: c.total_amount_cents.map(Cents::new),
                    unit_amount: c.unit_amount(&u.id).map(Cents::new),
                    enabled: c.enabled,
                    key: c.key(),
                })
                .collect();
            let basis = UnitBasis {
                shares: u.shares,
                total_shares: owner.total_shares,
                unit_count,
            };
            let monthly: Cents = prepayments
                .iter()
                .filter(|p| p.unit_id == u.id)
                .map(|p| Cents::new(p.monthly_amount_cents))
                .sum();
            let statement = unit_statement(&positions, &basis, monthly, range, &DEFAULT_MONTH_RULE)
                .map_err(|e| ApiError::invalid_shares(format!("Einheit „{}“: {e}", u.name)))?;
            Ok(UnitTotalJson {
                unit_id: u.id.clone(),
                total_unit_costs_cents: statement.unit_costs.value(),
                total_prepayment_cents: statement.prepayment.value(),
                difference_cents: statement.difference().value(),
                months: statement.months,
            })
        })
        .collect()
}
