//! Review progress of a billing period.

/// Review state of one cost position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CostReview {
    /// Disabled positions do not count (they are skipped in the calculation too).
    pub enabled: bool,
    /// Whether the position has been reviewed.
    pub reviewed: bool,
}

/// How many positions of a period are reviewed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewProgress {
    /// Enabled cost positions plus prepayments.
    pub total: usize,
    /// Reviewed ones among them.
    pub reviewed: usize,
}

impl ReviewProgress {
    /// Counts enabled cost positions and all prepayments (`reviewProgress` /
    /// `getUnreviewedCount` in the old code).
    pub fn count(
        costs: impl IntoIterator<Item = CostReview>,
        prepayments_reviewed: impl IntoIterator<Item = bool>,
    ) -> Self {
        let mut progress = ReviewProgress {
            total: 0,
            reviewed: 0,
        };
        let enabled_costs = costs.into_iter().filter(|c| c.enabled).map(|c| c.reviewed);
        for reviewed in enabled_costs.chain(prepayments_reviewed) {
            progress.total += 1;
            progress.reviewed += usize::from(reviewed);
        }
        progress
    }

    /// Positions still to review (`unreviewedCount` in the API). Saturates at 0 for
    /// hand-built values with more reviewed than total positions.
    pub fn unreviewed(&self) -> usize {
        self.total.saturating_sub(self.reviewed)
    }

    /// Whether there is at least one position and all are reviewed.
    pub fn all_reviewed(&self) -> bool {
        self.total > 0 && self.reviewed == self.total
    }
}
