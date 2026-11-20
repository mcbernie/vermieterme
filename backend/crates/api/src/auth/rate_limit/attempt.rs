//! A reserved attempt of the rate limiter and how it is booked.

use super::{Outcome, Policy, RateLimiter};

/// A reserved attempt. Book it with [`Attempt::failed`] or [`Attempt::succeeded`];
/// dropping it unbooked (other errors, cancelled request) frees the reservation without
/// counting a failure. An untracked attempt (full store) books nothing.
#[derive(Debug)]
#[must_use = "an attempt counts against the limit until it is booked or dropped"]
pub struct Attempt<'a> {
    pub(super) limiter: &'a RateLimiter,
    pub(super) policy: Policy,
    /// Key and sequence number of the entry; `None` once booked or if untracked.
    pub(super) slot: Option<(String, u64)>,
}

impl Attempt<'_> {
    /// Books a failure; locks the key when the limit is reached.
    pub fn failed(mut self) {
        self.settle(Outcome::Failed);
    }

    /// Books a success: per-account keys are cleared, per-IP keys only lose the
    /// reservation.
    pub fn succeeded(mut self) {
        self.settle(Outcome::Succeeded);
    }

    /// Books `outcome` once.
    fn settle(&mut self, outcome: Outcome) {
        if let Some((key, seq)) = self.slot.take() {
            self.limiter.settle(&self.policy, &key, seq, outcome);
        }
    }
}

impl Drop for Attempt<'_> {
    /// Frees an unbooked reservation.
    fn drop(&mut self) {
        self.settle(Outcome::Released);
    }
}
