//! Units (Einheiten) of a property.

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};
use vermieterme_domain::billing::UnitBasis;

use self::billings::detach_from_billings;
pub(crate) use self::locks::{lock, lock_of_property};
use super::{Checked, Rejection, property};
use crate::error::{DbError, DbResult};
use crate::row::{new_id, now_millis};
use crate::scope::AccountScope;
use crate::tx::in_transaction_read_committed;

/// Allowed values of [`Unit::kind`] (#2).
pub const KINDS: [&str; 4] = ["apartment", "garage", "carport", "other"];

/// A unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    /// Primary key.
    pub id: String,
    /// The property it belongs to.
    pub property_id: String,
    /// Name (e.g. "Wohnung 3").
    pub name: String,
    /// Floor (free text).
    pub floor: String,
    /// Co-ownership shares (MEA) of the unit.
    pub shares: i64,
    /// One of [`KINDS`].
    pub kind: String,
    /// Creation time (Unix ms).
    pub created_at: i64,
    /// Last change (Unix ms).
    pub updated_at: i64,
}

/// Values of create and update (already validated and trimmed by the caller).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitInput {
    /// The property the unit belongs to.
    pub property_id: String,
    /// Name.
    pub name: String,
    /// Floor.
    pub floor: String,
    /// Shares, at least 0.
    pub shares: i64,
    /// One of [`KINDS`].
    pub kind: String,
}

/// Column list read by [`from_row`].
macro_rules! columns {
    () => {
        "id, property_id, name, floor, shares, kind, created_at, updated_at"
    };
}

// After `columns!`, which the lock queries expand.
mod billings;
mod locks;

/// Maps a row selected with `columns!()`.
pub(super) fn from_row(row: &AnyRow) -> DbResult<Unit> {
    Ok(Unit {
        id: row.try_get("id")?,
        property_id: row.try_get("property_id")?,
        name: row.try_get("name")?,
        floor: row.try_get("floor")?,
        shares: row.try_get("shares")?,
        kind: row.try_get("kind")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

/// Lists the units of the account (optionally of one property), newest first.
pub async fn list(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    property_id: Option<&str>,
) -> DbResult<Vec<Unit>> {
    let sql = match property_id {
        Some(_) => concat!(
            "SELECT ",
            columns!(),
            " FROM unit WHERE account_id = ? AND property_id = ? ORDER BY created_at DESC, id DESC"
        ),
        None => concat!(
            "SELECT ",
            columns!(),
            " FROM unit WHERE account_id = ? ORDER BY created_at DESC, id DESC"
        ),
    };
    let mut query = sqlx::query(sql).bind(scope.account_id());
    if let Some(property_id) = property_id {
        query = query.bind(property_id);
    }
    let rows = query.fetch_all(&mut *conn).await?;
    rows.iter().map(from_row).collect()
}

/// Reads one unit of the account.
pub async fn get(conn: &mut AnyConnection, scope: &AccountScope, id: &str) -> DbResult<Unit> {
    let row = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM unit WHERE account_id = ? AND id = ?"
    ))
    .bind(scope.account_id())
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(DbError::NotFound)?;
    from_row(&row)
}

/// Fails with [`DbError::NotFound`] unless the unit exists in the account. Child tables
/// (tenant, rent change) call it first: the composite foreign key would also refuse a
/// foreign unit, but only as an opaque constraint error.
pub(super) async fn ensure(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
) -> DbResult<()> {
    sqlx::query("SELECT 1 AS present FROM unit WHERE account_id = ? AND id = ?")
        .bind(scope.account_id())
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?
        .map(|_| ())
        .ok_or(DbError::NotFound)
}

/// Checks the unit's shares against the (locked) total of its property.
async fn check_shares(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    input: &UnitInput,
) -> DbResult<Result<(), Rejection>> {
    let total_shares = property::lock(conn, scope, &input.property_id).await?;
    let basis = UnitBasis {
        shares: input.shares,
        total_shares,
        unit_count: 0,
    };
    Ok(basis
        .check_shares()
        .map_err(|_| Rejection::SharesAboveTotal))
}

/// Creates a unit. A foreign or unknown property is [`DbError::NotFound`]; shares above
/// the property's total are refused with [`Rejection::SharesAboveTotal`]. The property
/// is locked before the insert (lock order, see `repo::billing`).
pub async fn create(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    input: &UnitInput,
) -> Checked<Unit> {
    in_transaction_read_committed(conn, async |tx| {
        if let Err(rejection) = check_shares(tx, scope, input).await? {
            return Ok(Err(rejection));
        }
        let id = new_id();
        let now = now_millis();
        sqlx::query(
            "INSERT INTO unit (id, account_id, property_id, name, floor, shares, kind, \
             created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(scope.account_id())
        .bind(&input.property_id)
        .bind(&input.name)
        .bind(&input.floor)
        .bind(input.shares)
        .bind(&input.kind)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        get(tx, scope, &id).await.map(Ok)
    })
    .await
}

/// Replaces the values of a unit (the property may change). Same rules as [`create`];
/// moving it is refused while billings reference it ([`Rejection::UnitInBillings`]),
/// and its unit amounts that no longer count are deleted with the move (#117).
///
/// Lock order (see `repo::billing`): the current and the target property (sorted by
/// id), then the unit. Their ids come from a plain read first; the billing check and
/// the clean-up (`detach_from_billings`, categories before their rows) run after these
/// locks. Moved to a third property meanwhile -> [`DbError::Conflict`] (409, retry).
pub async fn update(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
    input: &UnitInput,
) -> Checked<Unit> {
    in_transaction_read_committed(conn, async |tx| {
        let owner = get(tx, scope, id).await?.property_id;
        let mut properties = [owner.as_str(), input.property_id.as_str()];
        properties.sort_unstable();
        property::lock(tx, scope, properties[0]).await?;
        if properties[1] != properties[0] {
            property::lock(tx, scope, properties[1]).await?;
        }
        let current = lock(tx, scope, id).await?;
        if current.property_id != owner && current.property_id != input.property_id {
            // Moved by someone else since the read above (rare). Locking its property now
            // would take step 1 after step 2 (deadlock risk): refuse, the caller retries.
            return Err(DbError::Conflict);
        }
        if let Err(rejection) = check_shares(tx, scope, input).await? {
            return Ok(Err(rejection));
        }
        if current.property_id != input.property_id
            && let Err(rejection) = detach_from_billings(tx, scope, id).await?
        {
            return Ok(Err(rejection));
        }
        sqlx::query(
            "UPDATE unit SET property_id = ?, name = ?, floor = ?, shares = ?, kind = ?, \
             updated_at = ? WHERE account_id = ? AND id = ?",
        )
        .bind(&input.property_id)
        .bind(&input.name)
        .bind(&input.floor)
        .bind(input.shares)
        .bind(&input.kind)
        .bind(now_millis())
        .bind(scope.account_id())
        .bind(id)
        .execute(&mut *tx)
        .await?;
        get(tx, scope, id).await.map(Ok)
    })
    .await
}

/// Deletes a unit with its tenants, rent changes and billing data (cascade). Its
/// property is locked first (lock order, see `repo::billing`), so a billing period
/// created at the same time either sees the unit or not, without a deadlock.
pub async fn delete(conn: &mut AnyConnection, scope: &AccountScope, id: &str) -> DbResult<()> {
    in_transaction_read_committed(conn, async |tx| {
        let owner = get(tx, scope, id).await?.property_id;
        property::lock(tx, scope, &owner).await?;
        let result = sqlx::query("DELETE FROM unit WHERE account_id = ? AND id = ?")
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
