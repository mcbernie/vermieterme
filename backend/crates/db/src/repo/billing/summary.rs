//! The list of billing periods with the counts shown in the overview.

use std::collections::HashMap;

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};
use vermieterme_domain::billing::{CostReview, ReviewProgress};

use super::period::{BillingPeriod, from_row};
use crate::error::DbResult;
use crate::row::flag;
use crate::scope::AccountScope;

/// A period in the list, with the number of cost positions and of open reviews.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodSummary {
    /// The period.
    pub period: BillingPeriod,
    /// Number of cost positions (enabled or not).
    pub cost_count: usize,
    /// Enabled cost positions and prepayments not yet reviewed (domain rule).
    pub unreviewed_count: usize,
}

/// Lists the periods of the account (optionally of one property), latest start first.
pub async fn list(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    property_id: Option<&str>,
) -> DbResult<Vec<PeriodSummary>> {
    let sql = match property_id {
        Some(_) => concat!(
            "SELECT ",
            period_columns!(),
            " FROM billing_period WHERE account_id = ? AND property_id = ? \
             ORDER BY start_date DESC, id DESC"
        ),
        None => concat!(
            "SELECT ",
            period_columns!(),
            " FROM billing_period WHERE account_id = ? ORDER BY start_date DESC, id DESC"
        ),
    };
    let mut query = sqlx::query(sql).bind(scope.account_id());
    if let Some(property_id) = property_id {
        query = query.bind(property_id);
    }
    let periods = query
        .fetch_all(&mut *conn)
        .await?
        .iter()
        .map(from_row)
        .collect::<DbResult<Vec<_>>>()?;
    let mut flags = review_flags(conn, scope, None).await?;
    Ok(periods
        .into_iter()
        .map(|period| {
            let (costs, prepayments) = flags.remove(&period.id).unwrap_or_default();
            summarize(period, &costs, prepayments)
        })
        .collect())
}

/// One period of the account with its counts (the answer of create and change);
/// an unknown or foreign period is [`DbError::NotFound`](crate::DbError::NotFound).
pub async fn get(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    period_id: &str,
) -> DbResult<PeriodSummary> {
    let period = super::period::get(conn, scope, period_id).await?;
    let mut flags = review_flags(conn, scope, Some(period_id)).await?;
    let (costs, prepayments) = flags.remove(period_id).unwrap_or_default();
    Ok(summarize(period, &costs, prepayments))
}

/// The counts of one period from its review flags (domain rule).
fn summarize(period: BillingPeriod, costs: &[CostReview], prepayments: Vec<bool>) -> PeriodSummary {
    let progress = ReviewProgress::count(costs.iter().copied(), prepayments);
    PeriodSummary {
        cost_count: costs.len(),
        unreviewed_count: progress.unreviewed(),
        period,
    }
}

/// Review flags of the costs and prepayments of the account (or of one period), by
/// period id.
async fn review_flags(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    period_id: Option<&str>,
) -> DbResult<HashMap<String, (Vec<CostReview>, Vec<bool>)>> {
    let mut flags: HashMap<String, (Vec<CostReview>, Vec<bool>)> = HashMap::new();
    let costs = rows(
        conn,
        scope,
        period_id,
        [
            "SELECT billing_period_id, enabled, reviewed FROM cost WHERE account_id = ?",
            "SELECT billing_period_id, enabled, reviewed FROM cost WHERE account_id = ? \
             AND billing_period_id = ?",
        ],
    )
    .await?;
    for row in &costs {
        let review = CostReview {
            enabled: flag(row, "enabled")?,
            reviewed: flag(row, "reviewed")?,
        };
        let entry = flags.entry(row.try_get("billing_period_id")?).or_default();
        entry.0.push(review);
    }
    let prepayments = rows(
        conn,
        scope,
        period_id,
        [
            "SELECT billing_period_id, reviewed FROM prepayment WHERE account_id = ?",
            "SELECT billing_period_id, reviewed FROM prepayment WHERE account_id = ? \
             AND billing_period_id = ?",
        ],
    )
    .await?;
    for row in &prepayments {
        let entry = flags.entry(row.try_get("billing_period_id")?).or_default();
        entry.1.push(flag(row, "reviewed")?);
    }
    Ok(flags)
}

/// Runs `[all, one]`: the first query for the whole account, the second (with an extra
/// period id parameter) for one period.
async fn rows(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    period_id: Option<&str>,
    [all, one]: [&'static str; 2],
) -> DbResult<Vec<AnyRow>> {
    let query = match period_id {
        Some(id) => sqlx::query(one).bind(scope.account_id()).bind(id),
        None => sqlx::query(all).bind(scope.account_id()),
    };
    Ok(query.fetch_all(&mut *conn).await?)
}
