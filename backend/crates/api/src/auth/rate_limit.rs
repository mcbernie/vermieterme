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
//! callers cap e-mail keys at `EMAIL_MAX`), each policy has its own store of at most
//! [`MAX_ENTRIES`] keys, and expired keys are removed by the periodic housekeeping
//! ([`RateLimiter::prune`]), never on the request path.
//!
//! A full store evicts only idle keys: a locked key or one with running attempts is never
//! pushed out, so flooding new keys cannot reset a lock (#101). If nothing is evictable,
//! the new key gets an **untracked** attempt (no entry; it is not limited by this policy)
//! instead of a refusal – a full store must not lock everybody out. Login then still
//! counts against the address limit; filling that store with locked keys would take
//! millions of password checks. Separate stores keep cheap keys of one policy (invalid
//! links) from crowding out another (login).

mod attempt;
mod store;

use std::collections::HashMap;
use std::net::{IpAddr, Ipv6Addr};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub use attempt::Attempt;
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

/// Most keys kept in memory per policy; a new key beyond that evicts the oldest idle one.
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

    /// Whether the entry may be evicted from a full store: not locked, nothing running.
    fn idle(&self, now: Instant) -> bool {
        self.in_flight == 0 && self.locked_until.is_none_or(|until| until <= now)
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
    /// Neither (e.g. invalid input, internal error, request cancelled before the check).
    Released,
}

/// The stores of all policies, by [`Policy::prefix`].
type Stores = HashMap<&'static str, Store<Entry>>;

/// Shared limiter (inside `AppState`).
#[derive(Debug)]
pub struct RateLimiter {
    stores: Mutex<Stores>,
    capacity: usize,
}

impl Default for RateLimiter {
    /// A limiter for [`MAX_ENTRIES`] keys.
    fn default() -> Self {
        Self::with_capacity(MAX_ENTRIES)
    }
}

impl RateLimiter {
    /// A limiter for at most `capacity` keys per policy.
    fn with_capacity(capacity: usize) -> Self {
        Self {
            stores: Mutex::new(HashMap::new()),
            capacity,
        }
    }

    /// Reserves one attempt for `key`: `Err(retry_after)` if the key is locked or its
    /// budget is used up by failures and running attempts; else the reservation, which
    /// counts against the limit until it is booked or dropped. If the policy's store is
    /// full of locked or busy keys, the attempt is untracked (see module docs).
    pub fn begin(&self, policy: &Policy, key: &str) -> Result<Attempt<'_>, Duration> {
        let now = Instant::now();
        let mut stores = self.lock();
        let store = stores
            .entry(policy.prefix)
            .or_insert_with(|| Store::with_capacity(self.capacity));
        let Some((seq, entry)) = store.entry(key, || Entry::new(now), |e| e.idle(now)) else {
            tracing::warn!(
                policy = policy.prefix,
                "Speicher des Rate-Limits voll, Versuch ohne Zählung"
            );
            return Ok(Attempt {
                limiter: self,
                policy: *policy,
                slot: None,
                armed: false,
            });
        };
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
            slot: Some((key.to_owned(), seq)),
            armed: false,
        })
    }

    /// Removes expired entries (housekeeping task).
    pub fn prune(&self) {
        let now = Instant::now();
        for store in self.lock().values_mut() {
            prune(store, now);
        }
    }

    /// Number of stored keys over all policies (tests).
    #[cfg(test)]
    fn len(&self) -> usize {
        self.lock().values().map(Store::len).sum()
    }

    /// Books the outcome of a reservation of `key`. Only the incarnation `seq` the
    /// reservation was made on is touched; a later entry of the same key is not.
    fn settle(&self, policy: &Policy, key: &str, seq: u64, outcome: Outcome) {
        let now = Instant::now();
        let mut stores = self.lock();
        let Some(entries) = stores.get_mut(policy.prefix) else {
            return;
        };
        let Some(entry) = entries.get_mut_seq(key, seq) else {
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

    /// The stores; a poisoned lock is taken over (the data stays consistent per entry).
    fn lock(&self) -> MutexGuard<'_, Stores> {
        self.stores
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
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
