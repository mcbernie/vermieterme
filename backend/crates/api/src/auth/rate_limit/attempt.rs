//! A reserved attempt of the rate limiter and how it is booked.

use super::{Outcome, Policy, RateLimiter};

/// A reserved attempt. Book it with [`Attempt::failed`], [`Attempt::succeeded`] or
/// [`Attempt::released`]. Dropping it unbooked (cancelled request) frees the reservation
/// without counting a failure – unless it was [armed](Attempt::arm): once the password
/// check has started, a cancelled attempt counts as failure (#104), so closing the
/// connection does not buy argon2 work outside the limit. An untracked attempt (full
/// store) books nothing.
#[derive(Debug)]
#[must_use = "an attempt counts against the limit until it is booked or dropped"]
pub struct Attempt<'a> {
    pub(super) limiter: &'a RateLimiter,
    pub(super) policy: Policy,
    /// Key and sequence number of the entry; `None` once booked or if untracked.
    pub(super) slot: Option<(String, u64)>,
    /// Whether dropping the attempt unbooked counts as failure.
    pub(super) armed: bool,
}

impl Attempt<'_> {
    /// Marks the start of the costly check (hash permit obtained): from now on, dropping
    /// the attempt unbooked counts as failure.
    pub fn arm(&mut self) {
        self.armed = true;
    }

    /// Books a failure; locks the key when the limit is reached.
    pub fn failed(mut self) {
        self.settle(Outcome::Failed);
    }

    /// Books a success: per-account keys are cleared, per-IP keys only lose the
    /// reservation.
    pub fn succeeded(mut self) {
        self.settle(Outcome::Succeeded);
    }

    /// Frees the reservation without counting a failure, even if armed (e.g. an internal
    /// error after the check).
    pub fn released(mut self) {
        self.settle(Outcome::Released);
    }

    /// Books `outcome` once.
    fn settle(&mut self, outcome: Outcome) {
        if let Some((key, seq)) = self.slot.take() {
            self.limiter.settle(&self.policy, &key, seq, outcome);
        }
    }
}

impl Drop for Attempt<'_> {
    /// Books an unbooked reservation: failure if armed, else just freed.
    fn drop(&mut self) {
        let outcome = if self.armed {
            Outcome::Failed
        } else {
            Outcome::Released
        };
        self.settle(outcome);
    }
}
