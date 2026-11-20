//! Shared set-up of the billing HTTP tests: a property with two units (40 and 60 of
//! 100 shares) and a period 2024 in account A, on top of the master data [`World`].

#![allow(dead_code)] // Not every test binary uses every helper.
#![allow(clippy::unwrap_used)]

use serde_json::{Value, json};

use super::masterdata_support::{World, id_of};

/// The billed objects of account A.
pub struct Billing {
    /// The property.
    pub property: String,
    /// The units (40 and 60 shares).
    pub units: [String; 2],
    /// The period 2024-01-01..2024-12-31.
    pub period: String,
}

impl Billing {
    /// `/api/billing-periods/{period}` plus `rest`.
    pub fn path(&self, rest: &str) -> String {
        format!("/api/billing-periods/{}{rest}", self.period)
    }
}

/// Creates property, units and the period for `cookie`.
pub async fn billing(w: &World, cookie: &str) -> Billing {
    let property = w.property(cookie).await;
    let units = [
        w.unit(cookie, &property, 40).await,
        w.unit(cookie, &property, 60).await,
    ];
    let body = json!({"propertyId": property, "startDate": "2024-01-01", "endDate": "2024-12-31"});
    let resp = w
        .expect(201, "POST", "/api/billing-periods", cookie, body)
        .await;
    Billing {
        period: id_of(&resp),
        property,
        units,
    }
}

/// Creates a cost category with `key`; returns its id.
pub async fn category(w: &World, cookie: &str, name: &str, key: &str) -> String {
    let body = json!({"name": name, "distributionKey": key});
    let resp = w
        .expect(201, "POST", "/api/cost-categories", cookie, body)
        .await;
    id_of(&resp)
}

/// The entry of `unit_id` in `totalsByUnit` of a detail.
pub fn totals_of<'a>(detail: &'a Value, unit_id: &str) -> &'a Value {
    detail["totalsByUnit"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["unitId"] == unit_id)
        .unwrap()
}
