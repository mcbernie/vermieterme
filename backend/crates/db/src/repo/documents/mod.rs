//! Documents of an account (P6): the rows that describe uploaded files. The files
//! themselves live below `UPLOAD_DIR/{account_id}/` under the stored [`Document::file_name`];
//! this crate never touches the file system.
//!
//! Every function takes a scope. A document may be assigned to a billing period, a tenant,
//! both or neither; the schema deletes it together with either of them (cascade), so the
//! callers of cascading deletes collect the stored file names first ([`files_of`]) and
//! remove the files after the commit (#106).
//!
//! # Locks
//!
//! Creating a document and collecting the files of a cascade follow the lock order of
//! `repo::billing`: first the **property** (step 1) the assignment belongs to, then, for a
//! tenant, its unit and tenant row. So an upload and a cascading delete of the same
//! property, unit, tenant or period run one after the other, and the delete always sees
//! the documents it takes along. Callers on MySQL use `READ COMMITTED`
//! ([`in_transaction_read_committed`](crate::in_transaction_read_committed)).
//!
//! The tenant app's visibility rule (#68, #71) is in [`tenant_view`].

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};

use crate::error::{DbError, DbResult};
use crate::row::{new_id, now_millis};
use crate::scope::AccountScope;
use crate::tx::in_transaction_read_committed;

pub use locks::{Owner, files_of};

/// Allowed values of [`NewDocument::category`].
pub const CATEGORIES: [&str; 4] = ["invoice", "receipt", "contract", "other"];

/// A document to store (validated by the caller; the file is already written).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDocument {
    /// Name of the stored file inside the account's upload directory (generated, never
    /// derived from the uploaded name).
    pub file_name: String,
    /// Name of the uploaded file as shown to users (sanitised, never used as a path).
    pub original_name: String,
    /// Content type detected from the file content.
    pub mime_type: String,
    /// Size in bytes.
    pub size_bytes: i64,
    /// One of [`CATEGORIES`].
    pub category: String,
    /// Assigned billing period.
    pub billing_period_id: Option<String>,
    /// Assigned tenant.
    pub tenant_id: Option<String>,
}

/// A stored document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// Primary key.
    pub id: String,
    /// Assigned billing period.
    pub billing_period_id: Option<String>,
    /// Assigned tenant.
    pub tenant_id: Option<String>,
    /// Name of the stored file inside the account's upload directory.
    pub file_name: String,
    /// Name shown to users.
    pub original_name: String,
    /// Stored content type.
    pub mime_type: String,
    /// Size in bytes.
    pub size_bytes: i64,
    /// One of [`CATEGORIES`].
    pub category: String,
    /// Upload time (Unix ms).
    pub created_at: i64,
}

/// Column list read by [`from_row`] (qualified, so joins can use it).
macro_rules! columns {
    () => {
        "d.id, d.billing_period_id, d.tenant_id, d.file_name, d.original_name, d.mime_type, \
         d.size_bytes, d.category, d.created_at"
    };
}

// After `columns!`, which the submodules expand.
mod locks;
pub mod tenant_view;

/// Maps a row selected with `columns!()`.
fn from_row(row: &AnyRow) -> DbResult<Document> {
    Ok(Document {
        id: row.try_get("id")?,
        billing_period_id: row.try_get("billing_period_id")?,
        tenant_id: row.try_get("tenant_id")?,
        file_name: row.try_get("file_name")?,
        original_name: row.try_get("original_name")?,
        mime_type: row.try_get("mime_type")?,
        size_bytes: row.try_get("size_bytes")?,
        category: row.try_get("category")?,
        created_at: row.try_get("created_at")?,
    })
}

/// Lists the documents of the account, newest first, optionally only those of a billing
/// period and/or a tenant (both filters apply together).
pub async fn list(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    billing_period_id: Option<&str>,
    tenant_id: Option<&str>,
) -> DbResult<Vec<Document>> {
    // Fixed statement: a missing filter binds NULL and matches every row.
    let rows = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM document d WHERE d.account_id = ? \
         AND (? IS NULL OR d.billing_period_id = ?) AND (? IS NULL OR d.tenant_id = ?) \
         ORDER BY d.created_at DESC, d.id DESC"
    ))
    .bind(scope.account_id())
    .bind(billing_period_id)
    .bind(billing_period_id)
    .bind(tenant_id)
    .bind(tenant_id)
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(from_row).collect()
}

/// Reads one document of the account.
pub async fn get(conn: &mut AnyConnection, scope: &AccountScope, id: &str) -> DbResult<Document> {
    let row = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM document d WHERE d.account_id = ? AND d.id = ?"
    ))
    .bind(scope.account_id())
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(DbError::NotFound)?;
    from_row(&row)
}

/// Stores a document. Its billing period and tenant must belong to the account (else
/// [`DbError::NotFound`]); both are locked in the lock order first (module
/// documentation), so a parallel cascading delete either sees the new row or removes
/// the assignment before it is checked here.
pub async fn create(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    new: &NewDocument,
) -> DbResult<Document> {
    in_transaction_read_committed(conn, async |tx| {
        locks::lock_assignment(
            tx,
            scope,
            new.billing_period_id.as_deref(),
            new.tenant_id.as_deref(),
        )
        .await?;
        let id = new_id();
        sqlx::query(
            "INSERT INTO document (id, account_id, billing_period_id, tenant_id, file_name, \
             original_name, mime_type, size_bytes, category, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(scope.account_id())
        .bind(new.billing_period_id.as_deref())
        .bind(new.tenant_id.as_deref())
        .bind(&new.file_name)
        .bind(&new.original_name)
        .bind(&new.mime_type)
        .bind(new.size_bytes)
        .bind(&new.category)
        .bind(now_millis())
        .execute(&mut *tx)
        .await?;
        get(tx, scope, &id).await
    })
    .await
}

/// Deletes a document and returns the name of its stored file, which the caller removes
/// after the commit.
pub async fn delete(conn: &mut AnyConnection, scope: &AccountScope, id: &str) -> DbResult<String> {
    in_transaction_read_committed(conn, async |tx| {
        let file_name = get(tx, scope, id).await?.file_name;
        let result = sqlx::query("DELETE FROM document WHERE account_id = ? AND id = ?")
            .bind(scope.account_id())
            .bind(id)
            .execute(&mut *tx)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound);
        }
        Ok(file_name)
    })
    .await
}
