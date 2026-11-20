//! Copying the positions of one billing period into a new one (`copyFromId`).

use sqlx::{AnyConnection, Row};

use super::period::{self, BillingPeriod};
use super::{Rejection, cost};
use crate::dialect::Dialect;
use crate::error::{DbError, DbResult};
use crate::row::{flag_value, new_id, now_millis};
use crate::scope::AccountScope;

/// Locks the source period (lock order step 1) and the categories of its costs (step 2).
/// A missing or foreign source is [`Rejection::SourceNotFound`], so the caller can tell
/// it apart from a missing property.
pub(super) async fn lock_source(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    source_id: &str,
) -> DbResult<Result<BillingPeriod, Rejection>> {
    let source = match period::lock(conn, scope, source_id).await {
        Ok(source) => source,
        Err(DbError::NotFound) => return Ok(Err(Rejection::SourceNotFound)),
        Err(other) => return Err(other),
    };
    let sql = Dialect::of(conn)?.pick(
        "SELECT id FROM cost_category WHERE account_id = ? AND id IN \
         (SELECT cost_category_id FROM cost WHERE account_id = ? AND billing_period_id = ?) \
         ORDER BY id",
        "SELECT id FROM cost_category WHERE account_id = ? AND id IN \
         (SELECT cost_category_id FROM cost WHERE account_id = ? AND billing_period_id = ?) \
         ORDER BY id FOR UPDATE",
    );
    sqlx::query(sql)
        .bind(scope.account_id())
        .bind(scope.account_id())
        .bind(&source.id)
        .fetch_all(&mut *conn)
        .await?;
    Ok(Ok(source))
}

/// Copies costs (amount, `enabled`, key override), their unit amounts and the
/// prepayments of `source_id` into `target_id`; everything copied is unreviewed and
/// starts at version 1. Unit amounts and prepayments are copied only for `units` (the
/// locked units of the property, lock order step 3).
pub(super) async fn copy_positions(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    source_id: &str,
    target_id: &str,
    units: &[String],
) -> DbResult<()> {
    let now = now_millis();
    for source in cost::list(conn, scope, source_id).await? {
        let id = new_id();
        sqlx::query(
            "INSERT INTO cost (id, account_id, billing_period_id, cost_category_id, \
             total_amount_cents, reviewed, enabled, distribution_key_override, version, \
             created_at, updated_at) VALUES (?, ?, ?, ?, ?, 0, ?, ?, 1, ?, ?)",
        )
        .bind(&id)
        .bind(scope.account_id())
        .bind(target_id)
        .bind(&source.cost_category_id)
        .bind(source.total_amount_cents)
        .bind(flag_value(source.enabled))
        .bind(source.override_key.map(|key| key.code()))
        .bind(now)
        .bind(now)
        .execute(&mut *conn)
        .await?;
        for amount in source
            .unit_amounts
            .iter()
            .filter(|a| units.contains(&a.unit_id))
        {
            cost::insert_unit_amount(conn, scope, &id, &amount.unit_id, amount.amount_cents)
                .await?;
        }
    }
    let prepayments = sqlx::query(
        "SELECT unit_id, monthly_amount_cents FROM prepayment \
         WHERE account_id = ? AND billing_period_id = ?",
    )
    .bind(scope.account_id())
    .bind(source_id)
    .fetch_all(&mut *conn)
    .await?;
    for row in &prepayments {
        let unit_id: String = row.try_get("unit_id")?;
        if !units.contains(&unit_id) {
            continue;
        }
        sqlx::query(
            "INSERT INTO prepayment (id, account_id, billing_period_id, unit_id, \
             monthly_amount_cents, reviewed, version, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, 0, 1, ?, ?)",
        )
        .bind(new_id())
        .bind(scope.account_id())
        .bind(target_id)
        .bind(&unit_id)
        .bind(row.try_get::<i64, _>("monthly_amount_cents")?)
        .bind(now)
        .bind(now)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}
