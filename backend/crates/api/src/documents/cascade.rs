//! Deletes that take documents along, with the removal of their files after the commit
//! (#106): deleting a document, a property, a unit, a tenant or a billing period.
//!
//! The stored file names are collected inside the transaction of the delete, under the
//! locks that keep uploads out ([`documents::files_of`]); the files are removed only
//! once the transaction has committed, so a rolled-back delete never loses a file.

use vermieterme_db::repo::documents::{self, Owner};
use vermieterme_db::repo::masterdata::Rejection;
use vermieterme_db::{AnyConnection, DbResult};

use super::changed_meanwhile;
use super::storage::remove_files;
use crate::auth::AccountCtx;
use crate::error::ApiResult;
use crate::masterdata::audited_read_committed;
use crate::routes::AppState;

/// Runs `delete` (which returns the stored file names it removed) with its audit entry
/// in one transaction (`READ COMMITTED`, the lock order of `repo::billing`), then
/// removes the files. `names` and `missing` as for the master data helpers.
pub(crate) async fn delete_with_files(
    state: &AppState,
    ctx: &AccountCtx,
    names: (&'static str, &'static str),
    missing: &'static str,
    entity_id: &str,
    delete: impl AsyncFnOnce(&mut AnyConnection) -> DbResult<Vec<String>>,
) -> ApiResult<()> {
    let files = audited_read_committed(
        state,
        ctx,
        names,
        missing,
        |_: &Vec<String>| entity_id.to_owned(),
        async |tx| Ok(Ok::<_, Rejection>(delete(tx).await?)),
    )
    .await
    .map_err(changed_meanwhile)?;
    remove_files(&state.config.upload_dir, ctx.scope.account_id(), &files).await;
    Ok(())
}

/// Deletes `owner` with `delete` and removes the files of the documents the cascade of
/// the schema takes along.
pub(crate) async fn delete_owner(
    state: &AppState,
    ctx: &AccountCtx,
    names: (&'static str, &'static str),
    missing: &'static str,
    owner: Owner<'_>,
    delete: impl AsyncFnOnce(&mut AnyConnection) -> DbResult<()>,
) -> ApiResult<()> {
    let id = match owner {
        Owner::Property(id) | Owner::Unit(id) | Owner::Tenant(id) | Owner::BillingPeriod(id) => id,
    };
    delete_with_files(state, ctx, names, missing, id, async |tx| {
        let files = documents::files_of(tx, &ctx.scope, owner).await?;
        delete(tx).await?;
        Ok(files)
    })
    .await
}
