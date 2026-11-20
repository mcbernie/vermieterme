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
    let which = property_id.map_or(Flags::Account, Flags::Property);
    let mut flags = review_flags(conn, scope, which).await?;
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
    let mut flags = review_flags(conn, scope, Flags::Period(period_id)).await?;
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

/// The three queries of [`rows`] for the review flags of `table` (alias `x`); `extra`
/// names further columns before `reviewed`.
macro_rules! flag_queries {
    ($table:literal, $extra:literal) => {
        [
            concat!(
                "SELECT x.billing_period_id, ",
                $extra,
                "x.reviewed FROM ",
                $table,
                " x WHERE x.account_id = ?"
            ),
            concat!(
                "SELECT x.billing_period_id, ",
                $extra,
                "x.reviewed FROM ",
                $table,
                " x JOIN billing_period p ON p.account_id = x.account_id \
                 AND p.id = x.billing_period_id WHERE x.account_id = ? AND p.property_id = ?"
            ),
            concat!(
                "SELECT x.billing_period_id, ",
                $extra,
                "x.reviewed FROM ",
                $table,
                " x WHERE x.account_id = ? AND x.billing_period_id = ?"
            ),
        ]
    };
}

/// Which periods [`review_flags`] reads.
#[derive(Clone, Copy)]
enum Flags<'a> {
    /// All periods of the account.
    Account,
    /// The periods of one property.
    Property(&'a str),
    /// One period.
    Period(&'a str),
}

/// Review flags of the costs and prepayments of the account, of one property or of one
/// period, by period id.
async fn review_flags(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    which: Flags<'_>,
) -> DbResult<HashMap<String, (Vec<CostReview>, Vec<bool>)>> {
    let mut flags: HashMap<String, (Vec<CostReview>, Vec<bool>)> = HashMap::new();
    for row in &rows(conn, scope, which, flag_queries!("cost", "x.enabled, ")).await? {
        let review = CostReview {
            enabled: flag(row, "enabled")?,
            reviewed: flag(row, "reviewed")?,
        };
        let entry = flags.entry(row.try_get("billing_period_id")?).or_default();
        entry.0.push(review);
    }
    for row in &rows(conn, scope, which, flag_queries!("prepayment", "")).await? {
        let entry = flags.entry(row.try_get("billing_period_id")?).or_default();
        entry.1.push(flag(row, "reviewed")?);
    }
    Ok(flags)
}

/// Runs `[account, property, period]` for `which`; the last two take the property or
/// period id as second parameter.
async fn rows(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    which: Flags<'_>,
    [account, property, period]: [&'static str; 3],
) -> DbResult<Vec<AnyRow>> {
    let query = match which {
        Flags::Account => sqlx::query(account).bind(scope.account_id()),
        Flags::Property(id) => sqlx::query(property).bind(scope.account_id()).bind(id),
        Flags::Period(id) => sqlx::query(period).bind(scope.account_id()).bind(id),
    };
    Ok(query.fetch_all(&mut *conn).await?)
}
