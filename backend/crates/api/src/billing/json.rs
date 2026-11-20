//! Response forms of the billing area (`docs/api-contract.md`, section 7): amounts in
//! cents, dates `YYYY-MM-DD`, timestamps ISO.

use serde::Serialize;
use vermieterme_db::repo::billing::{BillingPeriod, Cost, Prepayment};

use crate::error::json::timestamp;

/// A billing period.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeriodJson {
    id: String,
    property_id: String,
    start_date: String,
    end_date: String,
    billing_date: Option<String>,
    sent_date: Option<String>,
    paid_date: Option<String>,
    copied_from_id: Option<String>,
    created_at: String,
    updated_at: String,
}

impl From<BillingPeriod> for PeriodJson {
    /// Response form with ISO timestamps.
    fn from(p: BillingPeriod) -> Self {
        Self {
            id: p.id,
            property_id: p.property_id,
            start_date: p.start_date,
            end_date: p.end_date,
            billing_date: p.billing_date,
            sent_date: p.sent_date,
            paid_date: p.paid_date,
            copied_from_id: p.copied_from_id,
            created_at: timestamp(p.created_at),
            updated_at: timestamp(p.updated_at),
        }
    }
}

/// The amount of one unit in a cost position.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitAmountJson {
    unit_id: String,
    amount_cents: i64,
}

/// A cost position (`Cost` of the contract); `distributionKey` is the key in effect.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostJson {
    id: String,
    billing_period_id: String,
    cost_category_id: String,
    category_name: String,
    distribution_key: &'static str,
    distribution_key_override: Option<&'static str>,
    total_amount_cents: Option<i64>,
    unit_amounts: Vec<UnitAmountJson>,
    enabled: bool,
    reviewed: bool,
    version: i64,
}

impl From<Cost> for CostJson {
    /// Response form with the effective key and the stored unit amounts.
    fn from(c: Cost) -> Self {
        Self {
            distribution_key: c.key().code(),
            distribution_key_override: c.override_key.map(|k| k.code()),
            id: c.id,
            billing_period_id: c.billing_period_id,
            cost_category_id: c.cost_category_id,
            category_name: c.category_name,
            total_amount_cents: c.total_amount_cents,
            unit_amounts: c
                .unit_amounts
                .into_iter()
                .map(|a| UnitAmountJson {
                    unit_id: a.unit_id,
                    amount_cents: a.amount_cents,
                })
                .collect(),
            enabled: c.enabled,
            reviewed: c.reviewed,
            version: c.version,
        }
    }
}

/// A prepayment (`Prepayment` of the contract).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepaymentJson {
    id: String,
    billing_period_id: String,
    unit_id: String,
    monthly_amount_cents: i64,
    reviewed: bool,
    version: i64,
}

impl From<Prepayment> for PrepaymentJson {
    /// Response form.
    fn from(p: Prepayment) -> Self {
        Self {
            id: p.id,
            billing_period_id: p.billing_period_id,
            unit_id: p.unit_id,
            monthly_amount_cents: p.monthly_amount_cents,
            reviewed: p.reviewed,
            version: p.version,
        }
    }
}

/// A stored date `YYYY-MM-DD` as `DD.MM.YYYY` for messages; other text unchanged.
pub fn date_de(date: &str) -> String {
    match date.split('-').collect::<Vec<_>>().as_slice() {
        [year, month, day] => format!("{day}.{month}.{year}"),
        _ => date.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::date_de;

    #[test]
    fn german_dates() {
        assert_eq!(date_de("2024-12-31"), "31.12.2024");
        assert_eq!(date_de("x"), "x");
    }
}
