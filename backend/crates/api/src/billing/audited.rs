//! The transaction of a billing change with its audit entry, under `READ COMMITTED`
//! (#116).

use vermieterme_db::{AnyConnection, DbResult, in_transaction_read_committed};

use crate::auth::AccountCtx;
use crate::error::{ApiError, ApiResult, not_found_as};
use crate::routes::AppState;

/// Like [`audited_checked`](crate::masterdata::audited_checked), but the transaction runs
/// under `READ COMMITTED` on MySQL. The billing repositories rely on it (module
/// `repo::billing`, "No range locks beyond the property"): their own transaction is only
/// a savepoint of this one, and the isolation can only be chosen at its `BEGIN`. Used by
/// every billing change and by deleting a property (its cascade reaches the billing
/// tables).
pub(crate) async fn audited_read_committed<T, R: Into<ApiError>>(
    state: &AppState,
    ctx: &AccountCtx,
    (action, entity): (&'static str, &'static str),
    missing: &'static str,
    entity_id: impl FnOnce(&T) -> String,
    change: impl AsyncFnOnce(&mut AnyConnection) -> DbResult<Result<T, R>>,
) -> ApiResult<T> {
    let mut conn = state.conn().await?;
    let outcome = in_transaction_read_committed(&mut conn, async |tx| {
        let value = match change(tx).await? {
            Ok(value) => value,
            Err(rejection) => return Ok(Err(rejection)),
        };
        ctx.audit(tx, action, entity, Some(&entity_id(&value)))
            .await?;
        Ok(Ok(value))
    })
    .await
    .map_err(not_found_as(missing))?;
    outcome.map_err(Into::into)
}
