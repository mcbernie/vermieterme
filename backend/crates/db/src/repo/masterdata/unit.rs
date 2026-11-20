//! Units (Einheiten) of a property.

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};
use vermieterme_domain::billing::UnitBasis;

use super::{Checked, Rejection, property};
use crate::dialect::Dialect;
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

/// Maps a row selected with `columns!()`.
fn from_row(row: &AnyRow) -> DbResult<Unit> {
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

/// Replaces the values of a unit (the property may change). Same rules as [`create`].
///
/// Lock order (see `repo::billing`): the current and the target property (sorted by
/// id), then the unit. Their ids come from a plain read first, so the check for billing
/// data reads with locks ([`in_billings`]) and never from an older snapshot.
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
            // Moved by someone else since the read above (rare): lock its property too.
            property::lock(tx, scope, &current.property_id).await?;
        }
        if let Err(rejection) = check_shares(tx, scope, input).await? {
            return Ok(Err(rejection));
        }
        if current.property_id != input.property_id && in_billings(tx, scope, id).await? {
            return Ok(Err(Rejection::UnitInBillings));
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

/// Reads a unit and, on MySQL, locks its row until the end of the transaction (SQLite has
/// the write lock from `BEGIN IMMEDIATE`), in one statement. Needed before [`in_billings`]:
/// without the lock a concurrent insert into `prepayment`/`cost_unit_amount` could commit
/// between check and `UPDATE`. Billing code (P5) locks the same unit row through this
/// function before inserting prepayments or per-unit cost amounts, so both sides
/// serialize on it (decision 37).
pub(crate) async fn lock(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
) -> DbResult<Unit> {
    let sql = Dialect::of(conn)?.pick(
        concat!(
            "SELECT ",
            columns!(),
            " FROM unit WHERE account_id = ? AND id = ?"
        ),
        concat!(
            "SELECT ",
            columns!(),
            " FROM unit WHERE account_id = ? AND id = ? FOR UPDATE"
        ),
    );
    let row = sqlx::query(sql)
        .bind(scope.account_id())
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(DbError::NotFound)?;
    from_row(&row)
}

/// Ids of the units of a property, sorted, locked on MySQL until the end of the
/// transaction (lock order step 2, after the property: see `repo::billing`). Creating a
/// billing period and deleting a property take them, so moving a unit in or out waits.
pub(crate) async fn lock_of_property(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    property_id: &str,
) -> DbResult<Vec<String>> {
    let sql = Dialect::of(conn)?.pick(
        "SELECT id FROM unit WHERE account_id = ? AND property_id = ? ORDER BY id",
        "SELECT id FROM unit WHERE account_id = ? AND property_id = ? ORDER BY id FOR UPDATE",
    );
    let rows = sqlx::query(sql)
        .bind(scope.account_id())
        .bind(property_id)
        .fetch_all(&mut *conn)
        .await?;
    rows.iter()
        .map(|row| Ok(row.try_get::<String, _>("id")?))
        .collect()
}

/// Whether billings reference the unit (prepayments or per-unit cost amounts). Reads
/// with locks on MySQL, so it sees the latest committed rows even when the
/// transaction's snapshot is older.
async fn in_billings(conn: &mut AnyConnection, scope: &AccountScope, id: &str) -> DbResult<bool> {
    let dialect = Dialect::of(conn)?;
    let queries = [
        dialect.pick(
            "SELECT 1 AS used FROM prepayment WHERE account_id = ? AND unit_id = ? LIMIT 1",
            "SELECT 1 AS used FROM prepayment WHERE account_id = ? AND unit_id = ? LIMIT 1 \
             FOR UPDATE",
        ),
        dialect.pick(
            "SELECT 1 AS used FROM cost_unit_amount WHERE account_id = ? AND unit_id = ? LIMIT 1",
            "SELECT 1 AS used FROM cost_unit_amount WHERE account_id = ? AND unit_id = ? \
             LIMIT 1 FOR UPDATE",
        ),
    ];
    for sql in queries {
        let row = sqlx::query(sql)
            .bind(scope.account_id())
            .bind(id)
            .fetch_optional(&mut *conn)
            .await?;
        if row.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
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
