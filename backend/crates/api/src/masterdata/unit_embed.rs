//! The unit (with its property) embedded in tenant and rent-change responses, so the
//! user interface needs no second request (decision 38), and the property embed that
//! billing period summaries share.

use std::collections::HashMap;

use serde::Serialize;
use vermieterme_db::repo::masterdata::property::Property;
use vermieterme_db::repo::masterdata::unit_ref::{self, PropertyRef, UnitRef};

use crate::auth::AccountCtx;
use crate::error::ApiResult;
use crate::routes::AppState;

/// `property{id, street, zip, city}`: the one embedded form of a property (units of
/// tenants and rent changes, billing period summaries).
#[derive(Debug, Clone, Serialize)]
pub struct PropertyEmbed {
    id: String,
    street: String,
    zip: String,
    city: String,
}

impl From<&PropertyRef> for PropertyEmbed {
    /// Response form of the property of a unit reference.
    fn from(p: &PropertyRef) -> Self {
        Self {
            id: p.id.clone(),
            street: p.street.clone(),
            zip: p.zip.clone(),
            city: p.city.clone(),
        }
    }
}

impl From<Property> for PropertyEmbed {
    /// Response form of a full property.
    fn from(p: Property) -> Self {
        Self {
            id: p.id,
            street: p.street,
            zip: p.zip,
            city: p.city,
        }
    }
}

/// `unit{id, name, floor, shares, property}`.
#[derive(Debug, Clone, Serialize)]
pub struct UnitEmbed {
    id: String,
    name: String,
    floor: String,
    shares: i64,
    property: PropertyEmbed,
}

impl From<&UnitRef> for UnitEmbed {
    /// Response form.
    fn from(u: &UnitRef) -> Self {
        Self {
            id: u.id.clone(),
            name: u.name.clone(),
            floor: u.floor.clone(),
            shares: u.shares,
            property: PropertyEmbed::from(&u.property),
        }
    }
}

/// The units of the caller's account by id (the join is scoped).
pub async fn load(state: &AppState, ctx: &AccountCtx) -> ApiResult<HashMap<String, UnitRef>> {
    let mut conn = state.conn().await?;
    Ok(unit_ref::by_id(&mut conn, &ctx.scope).await?)
}

/// The embedded unit for `unit_id`, `None` only if it vanished in between.
pub fn lookup(units: &HashMap<String, UnitRef>, unit_id: &str) -> Option<UnitEmbed> {
    units.get(unit_id).map(UnitEmbed::from)
}
