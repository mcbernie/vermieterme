//! The tenant in responses (personal data: never logged), with the embedded unit.

use std::collections::HashMap;

use serde::Serialize;
use vermieterme_db::repo::masterdata::tenant::Tenant;
use vermieterme_db::repo::masterdata::unit_ref::UnitRef;

use super::unit_embed::{self, UnitEmbed};
use crate::error::json::timestamp;

/// A tenant in responses.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantJson {
    id: String,
    unit_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    unit: Option<UnitEmbed>,
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

impl TenantJson {
    /// Response form with ISO timestamps and the embedded unit with its property.
    pub fn new(t: Tenant, units: &HashMap<String, UnitRef>) -> Self {
        let d = t.data;
        Self {
            id: t.id,
            unit: unit_embed::lookup(units, &d.unit_id),
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
