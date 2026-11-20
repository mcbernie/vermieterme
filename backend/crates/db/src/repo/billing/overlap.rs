//! The overlap check of billing periods (decision 40).

use sqlx::{AnyConnection, Row};

use super::Rejection;
use crate::error::DbResult;
use crate::scope::AccountScope;

/// The first other period of the property that shares at least one day with
/// `start..=end` and is not `except`, as a [`Rejection::Overlap`]. Both ends are
/// inclusive, so a shared boundary day is an overlap too (decision 40: the boundary month
/// would otherwise count in both periods).
///
/// A plain read without locks (#116): the caller holds the property, which every writer
/// of its periods locks first, and runs under `READ COMMITTED`
/// ([`in_transaction_read_committed`](crate::in_transaction_read_committed)), so the
/// read sees the latest commit. A locking read would take next-key locks behind the
/// range, i.e. on the first period of the neighbouring property in the index (even of
/// another account), and deadlock with a change there.
pub(super) async fn overlap(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    property_id: &str,
    except: Option<&str>,
    start: &str,
    end: &str,
) -> DbResult<Option<Rejection>> {
    // Text dates `YYYY-MM-DD` compare like dates.
    let row = sqlx::query(
        "SELECT start_date, end_date FROM billing_period WHERE account_id = ? \
         AND property_id = ? AND id <> ? AND start_date <= ? AND end_date >= ? \
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
