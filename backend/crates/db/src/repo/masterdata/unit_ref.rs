//! Units with their property, read-only, for responses that embed them (tenants and rent
//! changes, decision 38).

use std::collections::HashMap;

use sqlx::{AnyConnection, Row};

use crate::error::DbResult;
use crate::scope::AccountScope;

/// The property part of a [`UnitRef`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyRef {
    /// Primary key.
    pub id: String,
    /// Street.
    pub street: String,
    /// Postal code.
    pub zip: String,
    /// City.
    pub city: String,
}

/// A unit with the data of its property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitRef {
    /// Primary key.
    pub id: String,
    /// Name.
    pub name: String,
    /// Floor.
    pub floor: String,
    /// Co-ownership shares.
    pub shares: i64,
    /// The property.
    pub property: PropertyRef,
}

/// All units of the account with their property, by unit id. Both tables are joined on
/// the account id, so only the scope's data appears.
pub async fn by_id(
    conn: &mut AnyConnection,
    scope: &AccountScope,
) -> DbResult<HashMap<String, UnitRef>> {
    let rows = sqlx::query(
        "SELECT u.id, u.name, u.floor, u.shares, p.id AS property_id, p.street, p.zip, p.city \
         FROM unit u JOIN property p ON p.account_id = u.account_id AND p.id = u.property_id \
         WHERE u.account_id = ?",
    )
    .bind(scope.account_id())
    .fetch_all(&mut *conn)
    .await?;
    let mut map = HashMap::with_capacity(rows.len());
    for row in &rows {
        let unit = UnitRef {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            floor: row.try_get("floor")?,
            shares: row.try_get("shares")?,
            property: PropertyRef {
                id: row.try_get("property_id")?,
                street: row.try_get("street")?,
                zip: row.try_get("zip")?,
                city: row.try_get("city")?,
            },
        };
        map.insert(unit.id.clone(), unit);
    }
    Ok(map)
}
