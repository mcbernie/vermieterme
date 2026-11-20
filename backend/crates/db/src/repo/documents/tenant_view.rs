//! Which documents a tenant sees in the tenant app (#68, #71, plan decision 20): those
//! assigned to the tenant, and those of a billing period of the tenant's property that
//! are assigned to no tenant. A document of another tenant (even in the same period) and
//! a document without any assignment stay invisible; for a single document both behave
//! like a missing one ([`DbError::NotFound`], 404).
//!
//! The one place of this rule: the list, the period detail and the download of the
//! tenant app (P7) all use these functions.

use sqlx::AnyConnection;

use super::{Document, from_row};
use crate::error::{DbError, DbResult};
use crate::scope::TenantScope;

/// The visibility condition on `document d`; binds account, tenant, account, property.
macro_rules! visible {
    () => {
        "d.account_id = ? AND (d.tenant_id = ? OR (d.tenant_id IS NULL AND \
         d.billing_period_id IN (SELECT bp.id FROM billing_period bp \
         WHERE bp.account_id = ? AND bp.property_id = ?)))"
    };
}

/// The documents the tenant may see, newest first.
pub async fn list(conn: &mut AnyConnection, scope: &TenantScope) -> DbResult<Vec<Document>> {
    let rows = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM document d WHERE ",
        visible!(),
        " ORDER BY d.created_at DESC, d.id DESC"
    ))
    .bind(scope.account_id())
    .bind(scope.tenant_id())
    .bind(scope.account_id())
    .bind(scope.property_id())
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(from_row).collect()
}

/// The visible documents of one billing period (period detail of the tenant app).
pub async fn list_of_period(
    conn: &mut AnyConnection,
    scope: &TenantScope,
    billing_period_id: &str,
) -> DbResult<Vec<Document>> {
    let rows = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM document d WHERE ",
        visible!(),
        " AND d.billing_period_id = ? ORDER BY d.created_at DESC, d.id DESC"
    ))
    .bind(scope.account_id())
    .bind(scope.tenant_id())
    .bind(scope.account_id())
    .bind(scope.property_id())
    .bind(billing_period_id)
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(from_row).collect()
}

/// One document if the tenant may see it; otherwise [`DbError::NotFound`].
pub async fn get(conn: &mut AnyConnection, scope: &TenantScope, id: &str) -> DbResult<Document> {
    let row = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM document d WHERE d.id = ? AND ",
        visible!()
    ))
    .bind(id)
    .bind(scope.account_id())
    .bind(scope.tenant_id())
    .bind(scope.account_id())
    .bind(scope.property_id())
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(DbError::NotFound)?;
    from_row(&row)
}
