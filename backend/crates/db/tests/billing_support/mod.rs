//! Shared set-up of the billing repository tests: an account with a property, units, a
//! cost category and a period.

#![allow(dead_code)] // Not every test binary uses every helper.
#![allow(clippy::unwrap_used)]
// Tests build scopes directly (no session layer here); see vermieterme_db::scope.
#![allow(clippy::disallowed_methods)]

use vermieterme_db::repo::account;
use vermieterme_db::repo::billing::{BillingPeriod, NewPeriod, period};
use vermieterme_db::repo::masterdata::cost_category::{self, CostCategoryInput};
use vermieterme_db::repo::masterdata::{property, unit};
use vermieterme_db::{AccountScope, AdminScope, Db};

/// One account with a property of 100 shares, two units (30 and 70 shares), a category
/// with key `mea` and one period 2024.
pub struct Setup {
    /// Scope of the account.
    pub scope: AccountScope,
    /// The property.
    pub property_id: String,
    /// The units.
    pub units: [String; 2],
    /// The category.
    pub category_id: String,
    /// The period 2024-01-01..2024-12-31.
    pub period: BillingPeriod,
}

/// A scope of a new account.
pub async fn scope(db: &Db, name: &str) -> AccountScope {
    let mut c = db.pool().acquire().await.unwrap();
    let admin = AdminScope::system();
    let a = account::create(&mut c, &admin, name).await.unwrap();
    admin.act_in(a.id)
}

/// Creates a property with 100 shares.
pub async fn property(db: &Db, scope: &AccountScope) -> String {
    let mut c = db.pool().acquire().await.unwrap();
    let input = property::PropertyInput {
        street: "Musterweg 1".into(),
        zip: "12345".into(),
        city: "Beispielstadt".into(),
        total_shares: 100,
    };
    property::create(&mut c, scope, &input).await.unwrap().id
}

/// Creates a unit with `shares` in the property.
pub async fn unit(db: &Db, scope: &AccountScope, property_id: &str, shares: i64) -> String {
    let mut c = db.pool().acquire().await.unwrap();
    let input = unit_input(property_id, shares);
    unit::create(&mut c, scope, &input)
        .await
        .unwrap()
        .unwrap()
        .id
}

/// Values of a unit in `property_id`.
pub fn unit_input(property_id: &str, shares: i64) -> unit::UnitInput {
    unit::UnitInput {
        property_id: property_id.into(),
        name: "Wohnung".into(),
        floor: "EG".into(),
        shares,
        kind: "apartment".into(),
    }
}

/// Creates a category with `key`.
pub async fn category(db: &Db, scope: &AccountScope, name: &str, key: &str) -> String {
    let mut c = db.pool().acquire().await.unwrap();
    let input = CostCategoryInput {
        name: name.into(),
        distribution_key: key.into(),
        sort_order: 0,
    };
    cost_category::create(&mut c, scope, &input)
        .await
        .unwrap()
        .id
}

/// Values of a new period of the property.
pub fn new_period(property_id: &str, start: &str, end: &str) -> NewPeriod {
    NewPeriod {
        property_id: property_id.into(),
        start_date: start.into(),
        end_date: end.into(),
        billing_date: None,
        copy_from_id: None,
    }
}

/// Builds [`Setup`] in a new account.
pub async fn setup(db: &Db, name: &str) -> Setup {
    let scope = scope(db, name).await;
    let property_id = property(db, &scope).await;
    let units = [
        unit(db, &scope, &property_id, 30).await,
        unit(db, &scope, &property_id, 70).await,
    ];
    let category_id = category(db, &scope, "Grundsteuer", "mea").await;
    let mut c = db.pool().acquire().await.unwrap();
    let input = new_period(&property_id, "2024-01-01", "2024-12-31");
    let period = period::create(&mut c, &scope, &input)
        .await
        .unwrap()
        .unwrap();
    Setup {
        scope,
        property_id,
        units,
        category_id,
        period,
    }
}
