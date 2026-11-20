//! Equal response time for public credential endpoints (login, invitation, reset): every
//! answer – success, failure or lock – is delayed to at least [`MIN_RESPONSE`] after the
//! request started, so timing does not reveal whether an address or link exists.

use std::time::Duration;

use tokio::time::Instant;

/// Minimum duration of an answer. Above the cost of one argon2id verification in a
/// release build, so the floor (not the hash) decides the time.
pub const MIN_RESPONSE: Duration = Duration::from_millis(300);

/// Start time of a request that must not answer early.
#[derive(Debug, Clone, Copy)]
pub struct Deadline(Instant);

impl Deadline {
    /// Starts the clock.
    pub fn start() -> Self {
        Self(Instant::now())
    }

    /// Sleeps until [`MIN_RESPONSE`] has passed since [`Deadline::start`].
    pub async fn wait(self) {
        tokio::time::sleep_until(self.0 + MIN_RESPONSE).await;
    }

    /// Waits, then passes `result` through (for `return deadline.finish(result).await`).
    pub async fn finish<T>(self, result: T) -> T {
        self.wait().await;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn waits_at_least_the_minimum() {
        let started = Instant::now();
        let value = Deadline::start().finish(7).await;
        assert_eq!(value, 7);
        assert!(started.elapsed() >= MIN_RESPONSE);
    }
}
