//! Billing periods (Abrechnungszeiträume) of a property.

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};

use super::{Checked, Rejection, copy, lock_property_units};
use crate::dialect::Dialect;
use crate::error::{DbError, DbResult};
use crate::repo::masterdata::property;
use crate::row::{new_id, now_millis};
use crate::scope::AccountScope;
use crate::tx::in_transaction;

/// A billing period. Dates are `YYYY-MM-DD`, both days inclusive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillingPeriod {
    /// Primary key.
    pub id: String,
    /// The billed property (cannot change).
    pub property_id: String,
    /// First day.
    pub start_date: String,
    /// Last day.
    pub end_date: String,
    /// Date of the statement.
    pub billing_date: Option<String>,
    /// Date the statement was sent.
    pub sent_date: Option<String>,
    /// Date the result was settled.
    pub paid_date: Option<String>,
    /// The period this one was copied from (same account; informational only).
    pub copied_from_id: Option<String>,
    /// Creation time (Unix ms).
    pub created_at: i64,
    /// Last change (Unix ms).
    pub updated_at: i64,
}

/// Values of a new period (validated by the caller: dates parse, end after start).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPeriod {
    /// The billed property.
    pub property_id: String,
    /// First day.
    pub start_date: String,
    /// Last day.
    pub end_date: String,
    /// Date of the statement.
    pub billing_date: Option<String>,
    /// Period to copy costs, unit amounts and prepayments from.
    pub copy_from_id: Option<String>,
}

/// Changes of a period: `None` keeps a value, `Some(None)` clears an optional date.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PeriodPatch {
    /// New first day.
    pub start_date: Option<String>,
    /// New last day.
    pub end_date: Option<String>,
    /// New statement date.
    pub billing_date: Option<Option<String>>,
    /// New sent date.
    pub sent_date: Option<Option<String>>,
    /// New paid date.
    pub paid_date: Option<Option<String>>,
}

/// Maps a row selected with `period_columns!()`.
pub(super) fn from_row(row: &AnyRow) -> DbResult<BillingPeriod> {
    Ok(BillingPeriod {
        id: row.try_get("id")?,
        property_id: row.try_get("property_id")?,
        start_date: row.try_get("start_date")?,
        end_date: row.try_get("end_date")?,
        billing_date: row.try_get("billing_date")?,
        sent_date: row.try_get("sent_date")?,
        paid_date: row.try_get("paid_date")?,
        copied_from_id: row.try_get("copied_from_id")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

/// Reads one period of the account.
pub async fn get(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
) -> DbResult<BillingPeriod> {
    let row = sqlx::query(concat!(
        "SELECT ",
        period_columns!(),
        " FROM billing_period WHERE account_id = ? AND id = ?"
    ))
    .bind(scope.account_id())
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(DbError::NotFound)?;
    from_row(&row)
}

/// Reads a period and, on MySQL, locks its row until the end of the transaction (lock
/// order step 1, see the module documentation of `billing`).
pub(super) async fn lock(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
) -> DbResult<BillingPeriod> {
    let sql = Dialect::of(conn)?.pick(
        concat!(
            "SELECT ",
            period_columns!(),
            " FROM billing_period WHERE account_id = ? AND id = ?"
        ),
        concat!(
            "SELECT ",
            period_columns!(),
            " FROM billing_period WHERE account_id = ? AND id = ? FOR UPDATE"
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

/// Creates a period, optionally as a copy of another period of the same property (costs
/// with `enabled` and key override, unit amounts and prepayments; all unreviewed), in
/// one transaction. Refused if the property has no units or another period overlaps.
pub async fn create(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    input: &NewPeriod,
) -> Checked<BillingPeriod> {
    in_transaction(conn, async |tx| {
        let source = match &input.copy_from_id {
            Some(source_id) => match copy::lock_source(tx, scope, source_id).await? {
                Ok(source) => Some(source),
                Err(rejection) => return Ok(Err(rejection)),
            },
            None => None,
        };
        if let Some(source) = &source
            && source.property_id != input.property_id
        {
            return Ok(Err(Rejection::SourceOtherProperty));
        }
        let units = lock_property_units(tx, scope, &input.property_id).await?;
        property::lock(tx, scope, &input.property_id).await?;
        if units.is_empty() {
            return Ok(Err(Rejection::NoUnits));
        }
        if let Some(rejection) = overlap(
            tx,
            scope,
            &input.property_id,
            None,
            &input.start_date,
            &input.end_date,
        )
        .await?
        {
            return Ok(Err(rejection));
        }
        let id = new_id();
        let now = now_millis();
        sqlx::query(
            "INSERT INTO billing_period (id, account_id, property_id, start_date, end_date, \
             billing_date, copied_from_id, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(scope.account_id())
        .bind(&input.property_id)
        .bind(&input.start_date)
        .bind(&input.end_date)
        .bind(&input.billing_date)
        .bind(source.as_ref().map(|s| s.id.as_str()))
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        if let Some(source) = &source {
            copy::copy_positions(tx, scope, &source.id, &id, &units).await?;
        }
        get(tx, scope, &id).await.map(Ok)
    })
    .await
}

/// The first other period of the property that shares at least one day with
/// `start..=end` and is not `except`, as a [`Rejection::Overlap`]. Adjacent periods do
/// not overlap: like before, a period may start on the last day of the previous one.
async fn overlap(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    property_id: &str,
    except: Option<&str>,
    start: &str,
    end: &str,
) -> DbResult<Option<Rejection>> {
    // Text dates `YYYY-MM-DD` compare like dates. Same condition as the old code
    // (`start < other.end AND end > other.start`).
    let row = sqlx::query(
        "SELECT start_date, end_date FROM billing_period WHERE account_id = ? \
         AND property_id = ? AND id <> ? AND start_date < ? AND end_date > ? \
         ORDER BY start_date LIMIT 1",
    )
    .bind(scope.account_id())
    .bind(property_id)
    .bind(except.unwrap_or(""))
    .bind(end)
    .bind(start)
    .fetch_optional(&mut *conn)
    .await?;
    row.map(|row| {
        Ok(Rejection::Overlap {
            start: row.try_get("start_date")?,
            end: row.try_get("end_date")?,
        })
    })
    .transpose()
}

/// Changes the dates of a period (the property cannot change). The resulting dates are
/// checked against each other and against the other periods of the property.
pub async fn update(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
    patch: &PeriodPatch,
) -> Checked<BillingPeriod> {
    in_transaction(conn, async |tx| {
        let current = lock(tx, scope, id).await?;
        let start = patch.start_date.as_ref().unwrap_or(&current.start_date);
        let end = patch.end_date.as_ref().unwrap_or(&current.end_date);
        if end <= start {
            return Ok(Err(Rejection::EndNotAfterStart));
        }
        property::lock(tx, scope, &current.property_id).await?;
        if let Some(rejection) =
            overlap(tx, scope, &current.property_id, Some(id), start, end).await?
        {
            return Ok(Err(rejection));
        }
        let keep = |new: &Option<Option<String>>, old: &Option<String>| match new {
            Some(value) => value.clone(),
            None => old.clone(),
        };
        sqlx::query(
            "UPDATE billing_period SET start_date = ?, end_date = ?, billing_date = ?, \
             sent_date = ?, paid_date = ?, updated_at = ? WHERE account_id = ? AND id = ?",
        )
        .bind(start)
        .bind(end)
        .bind(keep(&patch.billing_date, &current.billing_date))
        .bind(keep(&patch.sent_date, &current.sent_date))
        .bind(keep(&patch.paid_date, &current.paid_date))
        .bind(now_millis())
        .bind(scope.account_id())
        .bind(id)
        .execute(&mut *tx)
        .await?;
        get(tx, scope, id).await.map(Ok)
    })
    .await
}

/// Deletes a period with its costs, unit amounts, prepayments and document rows (cascade
/// of the schema). Stored files of the documents are not touched here (P6).
pub async fn delete(conn: &mut AnyConnection, scope: &AccountScope, id: &str) -> DbResult<()> {
    let result = sqlx::query("DELETE FROM billing_period WHERE account_id = ? AND id = ?")
        .bind(scope.account_id())
        .bind(id)
        .execute(&mut *conn)
        .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::NotFound);
    }
    Ok(())
}
