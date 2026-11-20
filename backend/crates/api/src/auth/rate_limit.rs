//! In-memory rate limits against guessing (decision 13: one process, one instance).
//!
//! A key (e.g. `login-ip:203.0.113.7`) collects failures within a window; once
//! [`Policy::max_failures`] is reached the key is locked for [`Policy::lockout`]. A
//! success clears per-account keys ([`RateLimiter::clear`]); per-IP keys only expire, so
//! one valid account does not reset an attacker's budget.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// Limits of one kind of key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// Key prefix, keeps kinds apart.
    pub prefix: &'static str,
    /// Failures allowed within [`Policy::window`] before the lock.
    pub max_failures: u32,
    /// Counting window.
    pub window: Duration,
    /// Duration of the lock.
    pub lockout: Duration,
}

/// Fifteen minutes, the window and lock of all policies.
const QUARTER_HOUR: Duration = Duration::from_secs(15 * 60);

/// Login attempts per client address.
pub const LOGIN_IP: Policy = policy("login-ip", 20);
/// Login attempts per e-mail address.
pub const LOGIN_EMAIL: Policy = policy("login-email", 5);
/// Redemption of invitation and reset links per client address.
pub const LINK_IP: Policy = policy("link-ip", 10);
/// Wrong current password at a password change, per user.
pub const PASSWORD_CHANGE: Policy = policy("password-change", 5);

/// Builds a policy with the common window and lock.
const fn policy(prefix: &'static str, max_failures: u32) -> Policy {
    Policy {
        prefix,
        max_failures,
        window: QUARTER_HOUR,
        lockout: QUARTER_HOUR,
    }
}

/// Number of keys above which expired entries are pruned on insert.
const PRUNE_ABOVE: usize = 10_000;

/// State of one key.
#[derive(Debug, Clone, Copy)]
struct Entry {
    failures: u32,
    window_start: Instant,
    locked_until: Option<Instant>,
}

/// Shared limiter (inside `AppState`).
#[derive(Debug, Default)]
pub struct RateLimiter {
    entries: Mutex<HashMap<String, Entry>>,
}

impl RateLimiter {
    /// `Err(retry_after)` if `key` is locked under `policy`.
    pub fn check(&self, policy: &Policy, key: &str) -> Result<(), Duration> {
        let now = Instant::now();
        let entries = self.lock();
        match entries
            .get(&full_key(policy, key))
            .and_then(|e| e.locked_until)
        {
            Some(until) if until > now => Err(until - now),
            _ => Ok(()),
        }
    }

    /// Records a failure; locks the key when the limit is reached.
    pub fn fail(&self, policy: &Policy, key: &str) {
        let now = Instant::now();
        let mut entries = self.lock();
        if entries.len() > PRUNE_ABOVE {
            prune(&mut entries, now);
        }
        let entry = entries.entry(full_key(policy, key)).or_insert(Entry {
            failures: 0,
            window_start: now,
            locked_until: None,
        });
        if now.duration_since(entry.window_start) > policy.window
            || entry.locked_until.is_some_and(|until| until <= now)
        {
            *entry = Entry {
                failures: 0,
                window_start: now,
                locked_until: None,
            };
        }
        entry.failures = entry.failures.saturating_add(1);
        if entry.failures >= policy.max_failures {
            entry.locked_until = Some(now + policy.lockout);
        }
    }

    /// Forgets a key (after a success).
    pub fn clear(&self, policy: &Policy, key: &str) {
        self.lock().remove(&full_key(policy, key));
    }

    /// Removes expired entries (housekeeping task).
    pub fn prune(&self) {
        prune(&mut self.lock(), Instant::now());
    }

    /// The map; a poisoned lock is taken over (the data stays consistent per entry).
    fn lock(&self) -> MutexGuard<'_, HashMap<String, Entry>> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Key in the map: policy prefix plus key.
fn full_key(policy: &Policy, key: &str) -> String {
    format!("{}:{key}", policy.prefix)
}

/// Drops entries whose lock ended and whose window is over (all windows are equal).
fn prune(entries: &mut HashMap<String, Entry>, now: Instant) {
    entries.retain(|_, e| {
        e.locked_until.is_some_and(|until| until > now)
            || now.duration_since(e.window_start) <= QUARTER_HOUR
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST: Policy = Policy {
        prefix: "t",
        max_failures: 3,
        window: Duration::from_secs(60),
        lockout: Duration::from_secs(60),
    };

    #[test]
    fn locks_after_max_failures() {
        let limiter = RateLimiter::default();
        for _ in 0..2 {
            limiter.fail(&TEST, "a");
            assert!(limiter.check(&TEST, "a").is_ok());
        }
        limiter.fail(&TEST, "a");
        let wait = limiter.check(&TEST, "a").unwrap_err();
        assert!(wait > Duration::from_secs(50));
        assert!(limiter.check(&TEST, "b").is_ok());
        assert!(limiter.check(&LOGIN_IP, "a").is_ok());
    }

    #[test]
    fn clear_resets() {
        let limiter = RateLimiter::default();
        for _ in 0..3 {
            limiter.fail(&TEST, "a");
        }
        limiter.clear(&TEST, "a");
        assert!(limiter.check(&TEST, "a").is_ok());
    }

    #[test]
    fn prune_keeps_active_entries() {
        let limiter = RateLimiter::default();
        limiter.fail(&TEST, "a");
        limiter.prune();
        assert_eq!(limiter.lock().len(), 1);
    }
}
