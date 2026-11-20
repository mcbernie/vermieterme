//! In-memory rate limits against guessing (decision 13: one process, one instance).
//!
//! A key (e.g. `login-ip:203.0.113.7`) collects failures within a window; once
//! [`Policy::max_failures`] is reached the key is locked for [`Policy::lockout`].
//!
//! An attempt is **reserved** before the work starts ([`RateLimiter::begin`]): checking
//! and counting happen in one step under the lock, so parallel requests cannot all pass
//! the check before the first failure is booked. Reserved, unfinished attempts count
//! against the limit; the returned [`Attempt`] books the outcome. A success clears
//! per-account keys ([`Policy::reset_on_success`]); per-IP keys only expire, so one
//! valid account does not reset an attacker's budget.
//!
//! Memory is bounded: keys are short ([`ip_key`] folds an IPv6 client into its /64, the
//! callers cap e-mail keys at `EMAIL_MAX`), the store holds at most [`MAX_ENTRIES`] keys
//! and evicts the oldest beyond that, and expired keys are removed by the periodic
//! housekeeping ([`RateLimiter::prune`]), never on the request path.

mod store;

use std::net::{IpAddr, Ipv6Addr};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use store::Store;

/// Limits of one kind of key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// Key prefix, keeps kinds apart.
    pub prefix: &'static str,
    /// Failures (plus reserved attempts) allowed within [`Policy::window`].
    pub max_failures: u32,
    /// Counting window.
    pub window: Duration,
    /// Duration of the lock.
    pub lockout: Duration,
    /// A success forgets the key (per-account keys); else it only ends the reservation.
    pub reset_on_success: bool,
}

/// Fifteen minutes, the window and lock of all policies.
const QUARTER_HOUR: Duration = Duration::from_secs(15 * 60);

/// Login attempts per client address.
pub const LOGIN_IP: Policy = policy("login-ip", 20, false);
/// Login attempts per e-mail address.
pub const LOGIN_EMAIL: Policy = policy("login-email", 5, true);
/// Redemption of invitation and reset links per client address.
pub const LINK_IP: Policy = policy("link-ip", 10, false);
/// Wrong current password at a password change, per user.
pub const PASSWORD_CHANGE: Policy = policy("password-change", 5, true);

/// Builds a policy with the common window and lock.
const fn policy(prefix: &'static str, max_failures: u32, reset_on_success: bool) -> Policy {
    Policy {
        prefix,
        max_failures,
        window: QUARTER_HOUR,
        lockout: QUARTER_HOUR,
        reset_on_success,
    }
}

/// `Retry-After` when the budget is taken by attempts still running (not a lock).
const BUSY_RETRY: Duration = Duration::from_secs(1);

/// Most keys kept in memory; a new key beyond that evicts the oldest one.
pub const MAX_ENTRIES: usize = 100_000;

/// Rate-limit key of a client address: IPv4 (also IPv4-mapped IPv6) per address, IPv6
/// per /64 network – one client usually controls a whole /64.
pub fn ip_key(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => v4.to_string(),
            None => {
                let network = u128::from(v6) & !u128::from(u64::MAX);
                format!("{}/64", Ipv6Addr::from(network))
            }
        },
    }
}

/// State of one key.
#[derive(Debug, Clone, Copy)]
struct Entry {
    failures: u32,
    in_flight: u32,
    window_start: Instant,
    locked_until: Option<Instant>,
}

impl Entry {
    /// A fresh entry whose window starts `now`.
    fn new(now: Instant) -> Self {
        Self {
            failures: 0,
            in_flight: 0,
            window_start: now,
            locked_until: None,
        }
    }

    /// Starts a new window if the lock ended or the window is over (keeps reservations).
    fn refresh(&mut self, policy: &Policy, now: Instant) {
        let lock_over = self.locked_until.is_some_and(|until| until <= now);
        let window_over =
            self.locked_until.is_none() && now.duration_since(self.window_start) > policy.window;
        if lock_over || window_over {
            *self = Self {
                in_flight: self.in_flight,
                ..Self::new(now)
            };
        }
    }
}

/// How a reserved attempt ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// Counts as failure (wrong password, invalid link).
    Failed,
    /// Success; clears per-account keys.
    Succeeded,
    /// Neither (e.g. invalid input, internal error, cancelled request).
    Released,
}

/// Shared limiter (inside `AppState`).
#[derive(Debug)]
pub struct RateLimiter {
    entries: Mutex<Store<Entry>>,
}

impl Default for RateLimiter {
    /// A limiter for [`MAX_ENTRIES`] keys.
    fn default() -> Self {
        Self::with_capacity(MAX_ENTRIES)
    }
}

impl RateLimiter {
    /// A limiter for at most `capacity` keys.
    fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Mutex::new(Store::with_capacity(capacity)),
        }
    }

    /// Reserves one attempt for `key`: `Err(retry_after)` if the key is locked or its
    /// budget is used up by failures and running attempts; else the reservation, which
    /// counts against the limit until it is booked or dropped.
    pub fn begin(&self, policy: &Policy, key: &str) -> Result<Attempt<'_>, Duration> {
        let now = Instant::now();
        let key = full_key(policy, key);
        let mut entries = self.lock();
        let entry = entries.entry(&key, || Entry::new(now));
        entry.refresh(policy, now);
        if let Some(until) = entry.locked_until {
            return Err(until - now);
        }
        if entry.failures.saturating_add(entry.in_flight) >= policy.max_failures {
            return Err(BUSY_RETRY);
        }
        entry.in_flight += 1;
        Ok(Attempt {
            limiter: self,
            policy: *policy,
            key,
            settled: false,
        })
    }

    /// Removes expired entries (housekeeping task).
    pub fn prune(&self) {
        prune(&mut self.lock(), Instant::now());
    }

    /// Books the outcome of a reservation of `key` (the entry may be gone after a prune).
    fn settle(&self, policy: &Policy, key: &str, outcome: Outcome) {
        let now = Instant::now();
        let mut entries = self.lock();
        let Some(entry) = entries.get_mut(key) else {
            return;
        };
        entry.in_flight = entry.in_flight.saturating_sub(1);
        match outcome {
            Outcome::Failed => {
                entry.refresh(policy, now);
                entry.failures = entry.failures.saturating_add(1);
                if entry.failures >= policy.max_failures {
                    entry.locked_until = Some(now + policy.lockout);
                }
            }
            Outcome::Succeeded if policy.reset_on_success => {
                if entry.in_flight == 0 {
                    entries.remove(key);
                } else {
                    *entry = Entry {
                        in_flight: entry.in_flight,
                        ..Entry::new(now)
                    };
                }
            }
            Outcome::Succeeded | Outcome::Released => {}
        }
    }

    /// The map; a poisoned lock is taken over (the data stays consistent per entry).
    fn lock(&self) -> MutexGuard<'_, Store<Entry>> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// A reserved attempt. Book it with [`Attempt::failed`] or [`Attempt::succeeded`];
/// dropping it unbooked (other errors, cancelled request) frees the reservation without
/// counting a failure.
#[derive(Debug)]
#[must_use = "an attempt counts against the limit until it is booked or dropped"]
pub struct Attempt<'a> {
    limiter: &'a RateLimiter,
    policy: Policy,
    key: String,
    settled: bool,
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
        if !self.settled {
            self.settled = true;
            self.limiter.settle(&self.policy, &self.key, outcome);
        }
    }
}

impl Drop for Attempt<'_> {
    /// Frees an unbooked reservation.
    fn drop(&mut self) {
        self.settle(Outcome::Released);
    }
}

/// Key in the map: policy prefix plus key.
fn full_key(policy: &Policy, key: &str) -> String {
    format!("{}:{key}", policy.prefix)
}

/// Drops idle entries whose lock ended and whose window is over (all windows are equal).
fn prune(entries: &mut Store<Entry>, now: Instant) {
    entries.retain(|e| {
        e.in_flight > 0
            || e.locked_until.is_some_and(|until| until > now)
            || now.duration_since(e.window_start) <= QUARTER_HOUR
    });
}

#[cfg(test)]
mod tests;
