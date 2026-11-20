//! The unit (with its property) embedded in tenant and rent-change responses, so the
//! user interface needs no second request (decision 38).

use std::collections::HashMap;

use serde::Serialize;
use vermieterme_db::repo::masterdata::unit_ref::{self, UnitRef};

use crate::auth::AccountCtx;
use crate::error::ApiResult;
use crate::routes::AppState;

/// `property{id, street, zip, city}`.
#[derive(Debug, Clone, Serialize)]
pub struct PropertyEmbed {
    id: String,
    street: String,
    zip: String,
    city: String,
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
            property: PropertyEmbed {
                id: u.property.id.clone(),
                street: u.property.street.clone(),
                zip: u.property.zip.clone(),
                city: u.property.city.clone(),
            },
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
