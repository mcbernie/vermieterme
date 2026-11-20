//! Row locks on units (lock order step 2, see `repo::billing`) and the check whether
//! billing data references a unit (decision 37).

use sqlx::{AnyConnection, Row};
use vermieterme_domain::DistributionKey;

use super::{Unit, from_row};
use crate::dialect::Dialect;
use crate::error::{DbError, DbResult};
use crate::scope::AccountScope;

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
/// Their callers run under `READ COMMITTED`, so the range read locks only these rows,
/// not the first unit of the next property in the index (#116).
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

/// Whether billings reference the unit: a prepayment, or a unit amount of a position
/// whose key in effect uses unit amounts (#117). Amounts kept from an earlier
/// `per_notice`/`per_attachment` key are ignored by the computation (decision 5) and
/// therefore do not block a move either; they stay stored.
///
/// Prepayments and unit amounts (with their positions) are read with locks on MySQL, so
/// the check also sees the latest commit when an outer transaction runs under
/// `REPEATABLE READ` (the master data API). The category key is a plain read: the
/// category row is lock order step 4, which a cost save holds while it waits for its
/// position (step 5); a category the read cannot see counts as in use.
pub(super) async fn in_billings(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
) -> DbResult<bool> {
    let dialect = Dialect::of(conn)?;
    let prepayment = dialect.pick(
        "SELECT 1 AS used FROM prepayment WHERE account_id = ? AND unit_id = ? LIMIT 1",
        "SELECT 1 AS used FROM prepayment WHERE account_id = ? AND unit_id = ? LIMIT 1 \
         FOR UPDATE",
    );
    let found = sqlx::query(prepayment)
        .bind(scope.account_id())
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?;
    if found.is_some() {
        return Ok(true);
    }
    let amounts = sqlx::query(dialect.pick(AMOUNTS_SQL, AMOUNTS_SQL_LOCKING))
        .bind(scope.account_id())
        .bind(id)
        .fetch_all(&mut *conn)
        .await?;
    for row in &amounts {
        let key = match row.try_get::<Option<String>, _>("distribution_key_override")? {
            Some(code) => Some(code),
            None => {
                category_key(conn, scope, &row.try_get::<String, _>("cost_category_id")?).await?
            }
        };
        let Some(code) = key else {
            return Ok(true);
        };
        let key = DistributionKey::from_code(&code)
            .map_err(|_| DbError::InvalidValue("distribution_key"))?;
        if key.uses_unit_amounts() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Unit amounts of the unit with the override and category of their position.
const AMOUNTS_SQL: &str = "SELECT c.distribution_key_override, c.cost_category_id \
     FROM cost_unit_amount a JOIN cost c ON c.account_id = a.account_id AND c.id = a.cost_id \
     WHERE a.account_id = ? AND a.unit_id = ?";

/// [`AMOUNTS_SQL`] locking the amounts and their positions (MySQL; both lock order step 5).
const AMOUNTS_SQL_LOCKING: &str = "SELECT c.distribution_key_override, c.cost_category_id \
     FROM cost_unit_amount a JOIN cost c ON c.account_id = a.account_id AND c.id = a.cost_id \
     WHERE a.account_id = ? AND a.unit_id = ? FOR UPDATE";

/// The stored key code of a cost category, `None` if the read does not see it.
async fn category_key(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
) -> DbResult<Option<String>> {
    let row =
        sqlx::query("SELECT distribution_key FROM cost_category WHERE account_id = ? AND id = ?")
            .bind(scope.account_id())
            .bind(id)
            .fetch_optional(&mut *conn)
            .await?;
    row.map(|row| Ok(row.try_get("distribution_key")?))
        .transpose()
}
