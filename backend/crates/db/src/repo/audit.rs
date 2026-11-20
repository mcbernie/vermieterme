//! Audit log of administrative actions (account switch, changes inside a foreign
//! account, account and user management).
//!
//! Entries hold only *who* (user id), *where* (account id), *what* (action, entity) and
//! *which* (entity id). Action and entity are `&'static str` so no field value, personal
//! data or secret can end up in the log by accident.

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};

use crate::error::DbResult;
use crate::row::{new_id, now_millis};
use crate::scope::AdminScope;

/// Maximum number of entries returned by [`list`] (API contract).
pub const MAX_PAGE: i64 = 100;

/// An entry to record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAuditEntry<'a> {
    /// Acting user (`None` = the system).
    pub user_id: Option<&'a str>,
    /// Affected account, if any.
    pub account_id: Option<&'a str>,
    /// Action code, e.g. `account.switch`, `user.lock`.
    pub action: &'static str,
    /// Entity type, e.g. `account`, `user`, `tenant`.
    pub entity: &'static str,
    /// Id of the affected entity.
    pub entity_id: Option<&'a str>,
}

/// A stored entry; `user_email` is joined for display and is `None` once the user is
/// deleted – intentional, the log keeps no copy of personal data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEntry {
    /// Primary key.
    pub id: String,
    /// Time (Unix ms).
    pub at: i64,
    /// Acting user.
    pub user_id: Option<String>,
    /// Current e-mail of the acting user.
    pub user_email: Option<String>,
    /// Affected account.
    pub account_id: Option<String>,
    /// Action code.
    pub action: String,
    /// Entity type.
    pub entity: String,
    /// Entity id.
    pub entity_id: Option<String>,
}

/// Position in the log for paging backwards: the `(at, id)` of the last entry of the
/// previous page. Several entries can share one millisecond, so the id breaks the tie;
/// a cursor of time alone would skip the rest of that millisecond at a page boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditCursor {
    /// Time of the last entry seen (Unix ms).
    pub at: i64,
    /// Id of the last entry seen.
    pub id: String,
}

impl AuditCursor {
    /// The cursor continuing after `entry` (pass the last entry of a page).
    pub fn after(entry: &AuditEntry) -> Self {
        Self {
            at: entry.at,
            id: entry.id.clone(),
        }
    }
}

/// Filter of [`list`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuditFilter {
    /// Only entries of this account.
    pub account_id: Option<String>,
    /// Only entries strictly before this position in the order `at DESC, id DESC`.
    pub before: Option<AuditCursor>,
}

/// Records an entry at the current time. Works on any connection, so it can join the
/// transaction of the change it documents.
pub async fn record(conn: &mut AnyConnection, entry: &NewAuditEntry<'_>) -> DbResult<String> {
    let id = new_id();
    sqlx::query(
        "INSERT INTO audit_log (id, at, user_id, account_id, action, entity, entity_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(now_millis())
    .bind(entry.user_id)
    .bind(entry.account_id)
    .bind(entry.action)
    .bind(entry.entity)
    .bind(entry.entity_id)
    .execute(&mut *conn)
    .await?;
    Ok(id)
}

/// One page of [`list`]: the entries and the cursor of the next page (`None` = last page).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditPage {
    /// At most [`MAX_PAGE`] entries, newest first.
    pub items: Vec<AuditEntry>,
    /// Cursor for the next page; set only if more entries exist.
    pub next: Option<AuditCursor>,
}

/// Shared part of the two list statements (select list and join).
macro_rules! list_select {
    () => {
        "SELECT l.id, l.at, l.user_id, u.email AS user_email, l.account_id, l.action, \
         l.entity, l.entity_id FROM audit_log l LEFT JOIN app_user u ON u.id = l.user_id WHERE "
    };
}

/// Cursor condition and order of the two list statements; a NULL cursor disables the
/// condition.
macro_rules! list_page {
    () => {
        "(? IS NULL OR l.at < ? OR (l.at = ? AND l.id < ?)) \
         ORDER BY l.at DESC, l.id DESC LIMIT ?"
    };
}

/// All entries; SQLite walks `audit_log_at_idx`.
const LIST_ALL: &str = concat!(list_select!(), list_page!());

/// Entries of one account. A separate static statement (instead of `? IS NULL OR
/// l.account_id = ?`) so SQLite can use `audit_log_account_at_idx`.
const LIST_ACCOUNT: &str = concat!(list_select!(), "l.account_id = ? AND ", list_page!());

/// Lists entries newest first (`at DESC, id DESC`), at most [`MAX_PAGE`] (admin). Reads
/// one entry more to know whether another page follows; [`AuditPage::next`] then points
/// after the last returned entry. Pages together return every entry exactly once.
pub async fn list(
    conn: &mut AnyConnection,
    _admin: &AdminScope,
    filter: &AuditFilter,
) -> DbResult<AuditPage> {
    let before_at = filter.before.as_ref().map(|c| c.at);
    let before_id = filter.before.as_ref().map(|c| c.id.as_str());
    let query = match filter.account_id.as_deref() {
        Some(account_id) => sqlx::query(LIST_ACCOUNT).bind(account_id),
        None => sqlx::query(LIST_ALL),
    };
    let rows = query
        .bind(before_at)
        .bind(before_at)
        .bind(before_at)
        .bind(before_id)
        .bind(MAX_PAGE + 1)
        .fetch_all(&mut *conn)
        .await?;
    let mut items = rows.iter().map(from_row).collect::<DbResult<Vec<_>>>()?;
    let page = usize::try_from(MAX_PAGE).unwrap_or(usize::MAX);
    let more = items.len() > page;
    items.truncate(page);
    let next = if more {
        items.last().map(AuditCursor::after)
    } else {
        None
    };
    Ok(AuditPage { items, next })
}

// `cfg(all(test, ...))` is not recognised as test code by clippy's in-test allowance.
#[cfg(all(test, feature = "testing"))]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{LIST_ACCOUNT, LIST_ALL};
    use crate::testing::TestDb;
    use sqlx::Row;

    /// SQLite query plan of `sql` as one string.
    async fn plan(db: &TestDb, sql: &'static str) -> String {
        let sql = sqlx::AssertSqlSafe(format!("EXPLAIN QUERY PLAN {sql}"));
        let rows = sqlx::query(sql)
            .bind("a")
            .fetch_all(db.db().pool())
            .await
            .unwrap();
        rows.iter()
            .map(|r| r.get::<String, _>("detail"))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Each statement walks its own index (no full scan, no temporary sort).
    #[tokio::test]
    async fn statements_use_their_index() {
        let db = TestDb::sqlite().await;
        let all = plan(&db, LIST_ALL).await;
        assert!(all.contains("audit_log_at_idx"), "{all}");
        let account = plan(&db, LIST_ACCOUNT).await;
        assert!(account.contains("audit_log_account_at_idx"), "{account}");
        assert!(!account.contains("TEMP B-TREE"), "{account}");
    }
}

/// Maps a row of [`list`].
fn from_row(row: &AnyRow) -> DbResult<AuditEntry> {
    Ok(AuditEntry {
        id: row.try_get("id")?,
        at: row.try_get("at")?,
        user_id: row.try_get("user_id")?,
        user_email: row.try_get("user_email")?,
        account_id: row.try_get("account_id")?,
        action: row.try_get("action")?,
        entity: row.try_get("entity")?,
        entity_id: row.try_get("entity_id")?,
    })
}
