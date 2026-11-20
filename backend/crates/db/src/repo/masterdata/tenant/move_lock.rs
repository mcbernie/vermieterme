//! Locks before a tenant moves to another unit (lock order of `repo::billing`, #120).
//!
//! A cascading delete of a unit or property collects the stored files of the documents
//! it takes along under the lock of the property (`documents::files_of`). A tenant moving
//! into that unit unlocked could slip in between the collection and the cascade, and its
//! documents would be deleted without their files. Hence the move locks the current and
//! the target property (step 1), then both units (step 2), then the tenant row.

use sqlx::{AnyConnection, Row};

use super::super::{property, unit};
use crate::dialect::Dialect;
use crate::error::{DbError, DbResult};
use crate::scope::AccountScope;

/// Locks the properties of `from_unit` and `to_unit` (sorted by id), then both units
/// (sorted by id), then the tenant row, and checks that nothing moved meanwhile: a unit
/// no longer in a locked property, or a tenant no longer in `from_unit`, is
/// [`DbError::Conflict`] (rare; the caller retries). A missing unit is
/// [`DbError::NotFound`]. Call inside a transaction under `READ COMMITTED` on MySQL.
pub(super) async fn lock_move(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    tenant_id: &str,
    from_unit: &str,
    to_unit: &str,
) -> DbResult<()> {
    // Plain reads first: the property ids are needed before their locks (step 1).
    let mut properties = vec![
        unit::get(conn, scope, from_unit).await?.property_id,
        unit::get(conn, scope, to_unit).await?.property_id,
    ];
    properties.sort();
    properties.dedup();
    for property_id in &properties {
        property::lock(conn, scope, property_id).await?;
    }
    let mut units = [from_unit, to_unit];
    units.sort_unstable();
    for unit_id in units {
        let locked = unit::lock(conn, scope, unit_id).await?;
        if !properties.contains(&locked.property_id) {
            return Err(DbError::Conflict);
        }
    }
    if locked_unit_of(conn, scope, tenant_id).await? != from_unit {
        return Err(DbError::Conflict);
    }
    Ok(())
}

/// The unit of a tenant; on MySQL the tenant row stays locked until the end of the
/// transaction.
async fn locked_unit_of(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    tenant_id: &str,
) -> DbResult<String> {
    let sql = Dialect::of(conn)?.pick(
        "SELECT unit_id FROM tenant WHERE account_id = ? AND id = ?",
        "SELECT unit_id FROM tenant WHERE account_id = ? AND id = ? FOR UPDATE",
    );
    let row = sqlx::query(sql)
        .bind(scope.account_id())
        .bind(tenant_id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(DbError::NotFound)?;
    Ok(row.try_get("unit_id")?)
}
