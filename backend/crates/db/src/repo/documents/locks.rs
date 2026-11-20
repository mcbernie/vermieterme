//! Locks around documents (lock order of `repo::billing`) and the stored file names a
//! cascading delete takes along (#106).

use sqlx::{AnyConnection, Row};

use crate::dialect::Dialect;
use crate::error::{DbError, DbResult};
use crate::repo::billing::period;
use crate::repo::masterdata::property;
use crate::scope::AccountScope;

/// The row whose delete cascades to documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner<'a> {
    /// A property: documents of its billing periods and of the tenants of its units.
    Property(&'a str),
    /// A unit: documents of its tenants.
    Unit(&'a str),
    /// A tenant: its documents.
    Tenant(&'a str),
    /// A billing period: its documents.
    BillingPeriod(&'a str),
}

/// Stored file names of the documents that deleting `owner` removes with it.
///
/// Call inside the transaction of the delete, before the delete itself. Locks the
/// property of `owner` first (and for a tenant its unit and tenant row), the same rows an
/// upload locks before it inserts, so no document can appear between this read and the
/// cascade. Missing or foreign `owner` -> [`DbError::NotFound`]; an owner that moved to
/// another property meanwhile -> [`DbError::Conflict`].
pub async fn files_of(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    owner: Owner<'_>,
) -> DbResult<Vec<String>> {
    let account = scope.account_id();
    let (sql, binds): (&str, Vec<&str>) = match owner {
        Owner::Property(id) => {
            property::lock(conn, scope, id).await?;
            (
                "SELECT d.file_name FROM document d WHERE d.account_id = ? AND (\
                 d.billing_period_id IN (SELECT bp.id FROM billing_period bp \
                 WHERE bp.account_id = ? AND bp.property_id = ?) \
                 OR d.tenant_id IN (SELECT t.id FROM tenant t JOIN unit u \
                 ON u.account_id = t.account_id AND u.id = t.unit_id \
                 WHERE t.account_id = ? AND u.property_id = ?))",
                vec![account, account, id, account, id],
            )
        }
        Owner::Unit(id) => {
            lock_unit_property(conn, scope, id).await?;
            (
                "SELECT d.file_name FROM document d WHERE d.account_id = ? AND \
                 d.tenant_id IN (SELECT t.id FROM tenant t WHERE t.account_id = ? AND t.unit_id = ?)",
                vec![account, account, id],
            )
        }
        Owner::Tenant(id) => {
            let property_id = tenant_property(conn, scope, id, false).await?;
            property::lock(conn, scope, &property_id).await?;
            if tenant_property(conn, scope, id, true).await? != property_id {
                return Err(DbError::Conflict);
            }
            (
                "SELECT d.file_name FROM document d WHERE d.account_id = ? AND d.tenant_id = ?",
                vec![account, id],
            )
        }
        Owner::BillingPeriod(id) => {
            let property_id = period::get(conn, scope, id).await?.property_id;
            property::lock(conn, scope, &property_id).await?;
            (
                "SELECT d.file_name FROM document d \
                 WHERE d.account_id = ? AND d.billing_period_id = ?",
                vec![account, id],
            )
        }
    };
    let mut query = sqlx::query(sql);
    for value in binds {
        query = query.bind(value);
    }
    let rows = query.fetch_all(&mut *conn).await?;
    rows.iter()
        .map(|row| Ok(row.try_get::<String, _>("file_name")?))
        .collect()
}

/// Locks the properties of a new document's billing period and tenant (sorted by id),
/// then the tenant's unit and row, and checks that both still exist in the account.
pub(super) async fn lock_assignment(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    billing_period_id: Option<&str>,
    tenant_id: Option<&str>,
) -> DbResult<()> {
    let mut properties = Vec::new();
    if let Some(id) = billing_period_id {
        properties.push(period::get(conn, scope, id).await?.property_id);
    }
    if let Some(id) = tenant_id {
        properties.push(tenant_property(conn, scope, id, false).await?);
    }
    properties.sort();
    properties.dedup();
    for property_id in &properties {
        property::lock(conn, scope, property_id).await?;
    }
    if let Some(id) = billing_period_id {
        // A period never changes its property; this re-read only sees a delete.
        period::get(conn, scope, id).await?;
    }
    if let Some(id) = tenant_id {
        let locked = tenant_property(conn, scope, id, true).await?;
        if !properties.contains(&locked) {
            return Err(DbError::Conflict);
        }
    }
    Ok(())
}

/// Locks the property of a unit (plain read of the property id, lock, re-read: a unit
/// move locks both properties, so after the lock the unit stays where it is).
async fn lock_unit_property(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    unit_id: &str,
) -> DbResult<()> {
    let sql = "SELECT property_id FROM unit WHERE account_id = ? AND id = ?";
    let read = async |conn: &mut AnyConnection| -> DbResult<String> {
        let row = sqlx::query(sql)
            .bind(scope.account_id())
            .bind(unit_id)
            .fetch_optional(&mut *conn)
            .await?
            .ok_or(DbError::NotFound)?;
        Ok(row.try_get("property_id")?)
    };
    let property_id = read(conn).await?;
    property::lock(conn, scope, &property_id).await?;
    if read(conn).await? != property_id {
        return Err(DbError::Conflict);
    }
    Ok(())
}

/// The property of a tenant's unit. With `locking`, the unit and tenant rows are locked
/// on MySQL until the end of the transaction (after the property: lock order steps 2 and
/// below), so the tenant cannot move while a document is added or collected.
async fn tenant_property(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    tenant_id: &str,
    locking: bool,
) -> DbResult<String> {
    let plain = "SELECT u.property_id FROM tenant t JOIN unit u \
                 ON u.account_id = t.account_id AND u.id = t.unit_id \
                 WHERE t.account_id = ? AND t.id = ?";
    let locked = "SELECT u.property_id FROM tenant t JOIN unit u \
                  ON u.account_id = t.account_id AND u.id = t.unit_id \
                  WHERE t.account_id = ? AND t.id = ? FOR UPDATE";
    let sql = if locking {
        Dialect::of(conn)?.pick(plain, locked)
    } else {
        plain
    };
    let row = sqlx::query(sql)
        .bind(scope.account_id())
        .bind(tenant_id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(DbError::NotFound)?;
    Ok(row.try_get("property_id")?)
}
