//! Properties (Objekte).

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};
use vermieterme_domain::billing::UnitBasis;

use super::{Checked, Rejection, unit};
use crate::dialect::Dialect;
use crate::error::{DbError, DbResult};
use crate::row::{new_id, now_millis};
use crate::scope::AccountScope;
use crate::tx::in_transaction;

/// A property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    /// Primary key.
    pub id: String,
    /// Street and house number.
    pub street: String,
    /// Postal code.
    pub zip: String,
    /// City.
    pub city: String,
    /// Co-ownership shares (MEA) of the whole property.
    pub total_shares: i64,
    /// Creation time (Unix ms).
    pub created_at: i64,
    /// Last change (Unix ms).
    pub updated_at: i64,
}

/// A property with its number of units (list).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertySummary {
    /// The property.
    pub property: Property,
    /// Number of units.
    pub unit_count: i64,
}

/// Values of create and update (already validated and trimmed by the caller).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyInput {
    /// Street and house number.
    pub street: String,
    /// Postal code.
    pub zip: String,
    /// City.
    pub city: String,
    /// Co-ownership shares of the whole property, at least 1.
    pub total_shares: i64,
}

/// Column list read by [`from_row`].
macro_rules! columns {
    () => {
        "id, street, zip, city, total_shares, created_at, updated_at"
    };
}

/// Maps a row selected with `columns!()`.
fn from_row(row: &AnyRow) -> DbResult<Property> {
    Ok(Property {
        id: row.try_get("id")?,
        street: row.try_get("street")?,
        zip: row.try_get("zip")?,
        city: row.try_get("city")?,
        total_shares: row.try_get("total_shares")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

/// Lists the properties of the account, newest first, with their unit counts.
pub async fn list(
    conn: &mut AnyConnection,
    scope: &AccountScope,
) -> DbResult<Vec<PropertySummary>> {
    let rows = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        ", (SELECT COUNT(*) FROM unit u WHERE u.account_id = property.account_id \
         AND u.property_id = property.id) AS unit_count \
         FROM property WHERE account_id = ? ORDER BY created_at DESC, id DESC"
    ))
    .bind(scope.account_id())
    .fetch_all(&mut *conn)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(PropertySummary {
                property: from_row(row)?,
                unit_count: row.try_get("unit_count")?,
            })
        })
        .collect()
}

/// Reads one property of the account.
pub async fn get(conn: &mut AnyConnection, scope: &AccountScope, id: &str) -> DbResult<Property> {
    let row = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM property WHERE account_id = ? AND id = ?"
    ))
    .bind(scope.account_id())
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(DbError::NotFound)?;
    from_row(&row)
}

/// Creates a property.
pub async fn create(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    input: &PropertyInput,
) -> DbResult<Property> {
    let id = new_id();
    let now = now_millis();
    sqlx::query(
        "INSERT INTO property (id, account_id, street, zip, city, total_shares, created_at, \
         updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(scope.account_id())
    .bind(&input.street)
    .bind(&input.zip)
    .bind(&input.city)
    .bind(input.total_shares)
    .bind(now)
    .bind(now)
    .execute(&mut *conn)
    .await?;
    get(conn, scope, &id).await
}

/// Replaces the values of a property. Refused with [`Rejection::TotalBelowUnitShares`]
/// if a unit would hold more shares than the new total (checked under the row lock, so
/// a concurrent unit change cannot slip through).
pub async fn update(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
    input: &PropertyInput,
) -> Checked<Property> {
    in_transaction(conn, async |tx| {
        lock(tx, scope, id).await?;
        let max: i64 = sqlx::query(
            "SELECT COALESCE(MAX(shares), 0) AS max_shares FROM unit \
             WHERE account_id = ? AND property_id = ?",
        )
        .bind(scope.account_id())
        .bind(id)
        .fetch_one(&mut *tx)
        .await?
        .try_get("max_shares")?;
        let basis = UnitBasis {
            shares: max,
            total_shares: input.total_shares,
            unit_count: 0,
        };
        if basis.check_shares().is_err() {
            return Ok(Err(Rejection::TotalBelowUnitShares));
        }
        sqlx::query(
            "UPDATE property SET street = ?, zip = ?, city = ?, total_shares = ?, updated_at = ? \
             WHERE account_id = ? AND id = ?",
        )
        .bind(&input.street)
        .bind(&input.zip)
        .bind(&input.city)
        .bind(input.total_shares)
        .bind(now_millis())
        .bind(scope.account_id())
        .bind(id)
        .execute(&mut *tx)
        .await?;
        get(tx, scope, id).await.map(Ok)
    })
    .await
}

/// Deletes a property with its units, tenants and billing data (cascade of the schema).
///
/// Locks the property and then its units before the cascade (lock order, see
/// `repo::billing`): the cascade reaches units and billing periods in an order of the
/// database's choosing, and a prepayment save holds its unit before its period.
pub async fn delete(conn: &mut AnyConnection, scope: &AccountScope, id: &str) -> DbResult<()> {
    in_transaction(conn, async |tx| {
        lock(tx, scope, id).await?;
        unit::lock_of_property(tx, scope, id).await?;
        let result = sqlx::query("DELETE FROM property WHERE account_id = ? AND id = ?")
            .bind(scope.account_id())
            .bind(id)
            .execute(&mut *tx)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    })
    .await
}

/// Reads the total shares of a property and, on MySQL, locks its row until the end of the
/// transaction (SQLite has taken the write lock with `BEGIN IMMEDIATE` already). Step 1
/// of the lock order (see `repo::billing`): unit and property changes start here, so
/// their share checks never run side by side; creating, changing or deleting a billing
/// period locks it against overlapping periods.
pub(crate) async fn lock(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
) -> DbResult<i64> {
    let sql = Dialect::of(conn)?.pick(
        "SELECT total_shares FROM property WHERE account_id = ? AND id = ?",
        "SELECT total_shares FROM property WHERE account_id = ? AND id = ? FOR UPDATE",
    );
    let row = sqlx::query(sql)
        .bind(scope.account_id())
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(DbError::NotFound)?;
    Ok(row.try_get("total_shares")?)
}
