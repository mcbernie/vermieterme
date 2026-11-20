//! Number of prepayment months in a billing period.
//!
//! The rule is a trait so it can be swapped later (decisions 4 and 17); every place
//! that needs months (billing detail, tenant app, PDF) must go through it instead of
//! counting days (the old PDF divided by 30.44, #69).

use crate::date::CalendarDate;

/// Decides how many monthly prepayments fall into a period.
pub trait MonthRule {
    /// Months between `start` and `end` (both inclusive). Always at least 1.
    fn months(&self, start: CalendarDate, end: CalendarDate) -> u32;
}

/// Default rule: every calendar month touched by the period counts in full.
///
/// `2025-01-15` to `2025-03-01` touches January, February and March, so 3 months.
/// An end before the start yields 1, as in the old `getMonthsInPeriod`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CalendarMonths;

impl MonthRule for CalendarMonths {
    fn months(&self, start: CalendarDate, end: CalendarDate) -> u32 {
        let months = end.month_index() - start.month_index() + 1;
        // Clamped to 1..=u32::MAX; the year range keeps it far below the upper bound.
        u32::try_from(months.max(1)).unwrap_or(u32::MAX)
    }
}

/// The rule in effect for the application.
pub const DEFAULT_MONTH_RULE: CalendarMonths = CalendarMonths;

/// Months of a period under [`DEFAULT_MONTH_RULE`].
pub fn billing_months(start: CalendarDate, end: CalendarDate) -> u32 {
    DEFAULT_MONTH_RULE.months(start, end)
}
