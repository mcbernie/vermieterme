//! Tests of the rate limiter.

use super::*;

const TEST: Policy = Policy {
    prefix: "t",
    max_failures: 3,
    window: Duration::from_secs(60),
    lockout: Duration::from_secs(60),
    reset_on_success: true,
};

const PER_IP: Policy = Policy {
    reset_on_success: false,
    ..TEST
};

/// Books `n` failures for `key`.
fn fail(limiter: &RateLimiter, policy: &Policy, key: &str, n: u32) {
    for _ in 0..n {
        limiter.begin(policy, key).unwrap().failed();
    }
}

#[test]
fn locks_after_max_failures() {
    let limiter = RateLimiter::default();
    fail(&limiter, &TEST, "a", 3);
    let wait = limiter.begin(&TEST, "a").unwrap_err();
    assert!(wait > Duration::from_secs(50));
    assert!(limiter.begin(&TEST, "b").is_ok());
    assert!(limiter.begin(&LOGIN_IP, "a").is_ok());
}

#[test]
fn running_attempts_count_against_the_limit() {
    let limiter = RateLimiter::default();
    let running: Vec<_> = (0..3).map(|_| limiter.begin(&TEST, "a").unwrap()).collect();
    assert_eq!(limiter.begin(&TEST, "a").unwrap_err(), BUSY_RETRY);
    for attempt in running {
        attempt.failed();
    }
    assert!(limiter.begin(&TEST, "a").unwrap_err() > BUSY_RETRY);
}

#[test]
fn dropped_attempt_frees_its_reservation() {
    let limiter = RateLimiter::default();
    for _ in 0..10 {
        drop(limiter.begin(&TEST, "a").unwrap());
    }
    fail(&limiter, &TEST, "a", 2);
    assert!(limiter.begin(&TEST, "a").is_ok());
}

#[test]
fn success_clears_per_account_keys_only() {
    let limiter = RateLimiter::default();
    fail(&limiter, &TEST, "a", 2);
    limiter.begin(&TEST, "a").unwrap().succeeded();
    fail(&limiter, &TEST, "a", 2);
    assert!(limiter.begin(&TEST, "a").is_ok());

    fail(&limiter, &PER_IP, "ip", 2);
    limiter.begin(&PER_IP, "ip").unwrap().succeeded();
    fail(&limiter, &PER_IP, "ip", 1);
    assert!(limiter.begin(&PER_IP, "ip").is_err());
}

#[test]
fn parallel_threads_get_at_most_the_limit() {
    let limiter = RateLimiter::default();
    let barrier = std::sync::Barrier::new(16);
    let passed = std::sync::atomic::AtomicU32::new(0);
    std::thread::scope(|s| {
        for _ in 0..16 {
            s.spawn(|| {
                barrier.wait();
                if let Ok(attempt) = limiter.begin(&TEST, "a") {
                    passed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    barrier_free_work();
                    attempt.failed();
                }
            });
        }
    });
    assert!(passed.into_inner() <= TEST.max_failures);
}

/// Stand-in for the password check between reservation and booking.
fn barrier_free_work() {
    std::thread::sleep(Duration::from_millis(20));
}

#[test]
fn prune_keeps_active_entries() {
    let limiter = RateLimiter::default();
    fail(&limiter, &TEST, "a", 1);
    let _running = limiter.begin(&TEST, "b").unwrap();
    limiter.prune();
    assert_eq!(limiter.len(), 2);
}

#[test]
fn ipv6_clients_are_keyed_per_64() {
    let ip = |s: &str| s.parse::<IpAddr>().unwrap();
    assert_eq!(ip_key(ip("2001:db8:1:2::1")), "2001:db8:1:2::/64");
    assert_eq!(
        ip_key(ip("2001:db8:1:2::1")),
        ip_key(ip("2001:db8:1:2:ffff:ffff:ffff:ffff"))
    );
    assert_ne!(ip_key(ip("2001:db8:1:2::1")), ip_key(ip("2001:db8:1:3::1")));
    assert_eq!(ip_key(ip("::ffff:192.0.2.7")), "192.0.2.7");
    assert_eq!(ip_key(ip("192.0.2.7")), "192.0.2.7");

    let limiter = RateLimiter::default();
    fail(&limiter, &TEST, &ip_key(ip("2001:db8::1")), 3);
    assert!(limiter.begin(&TEST, &ip_key(ip("2001:db8::abcd"))).is_err());
}

#[test]
fn key_count_is_capped() {
    let limiter = RateLimiter::with_capacity(5);
    for i in 0..50 {
        limiter.begin(&TEST, &format!("k{i}")).unwrap().failed();
    }
    assert_eq!(limiter.len(), 5);
    // The newest keys are kept.
    fail(&limiter, &TEST, "k49", 2);
    assert!(limiter.begin(&TEST, "k49").is_err());
}

#[test]
fn flooding_new_keys_does_not_reset_a_lock() {
    let limiter = RateLimiter::with_capacity(5);
    fail(&limiter, &TEST, "victim", 3);
    for i in 0..50 {
        limiter.begin(&TEST, &format!("flood{i}")).unwrap().failed();
    }
    assert!(limiter.begin(&TEST, "victim").unwrap_err() > BUSY_RETRY);
}

#[test]
fn full_store_of_locked_keys_does_not_lock_others_out() {
    let limiter = RateLimiter::with_capacity(3);
    for key in ["a", "b", "c"] {
        fail(&limiter, &TEST, key, 3);
    }
    // No entry for the new key, but no refusal either: untracked attempts.
    for _ in 0..10 {
        limiter.begin(&TEST, "new").unwrap().failed();
    }
    assert_eq!(limiter.len(), 3);
    assert!(limiter.begin(&TEST, "a").is_err());
}

#[test]
fn policies_have_separate_stores() {
    let other = Policy {
        prefix: "other",
        ..PER_IP
    };
    let limiter = RateLimiter::with_capacity(2);
    fail(&limiter, &TEST, "a", 3);
    fail(&limiter, &TEST, "b", 3);
    fail(&limiter, &other, "ip", 3);
    assert!(limiter.begin(&other, "ip").is_err());
}

#[test]
fn busy_key_survives_a_full_store() {
    let limiter = RateLimiter::with_capacity(1);
    let old = limiter.begin(&TEST, "a").unwrap();
    // "a" is busy, so "b" cannot evict it and stays untracked.
    drop(limiter.begin(&TEST, "b").unwrap());
    assert_eq!(limiter.len(), 1);
    let held: Vec<_> = (0..2).map(|_| limiter.begin(&TEST, "a").unwrap()).collect();
    assert_eq!(limiter.begin(&TEST, "a").unwrap_err(), BUSY_RETRY);
    old.failed();
    assert_eq!(limiter.begin(&TEST, "a").unwrap_err(), BUSY_RETRY);
    drop(held);
}

#[test]
fn settle_ignores_a_later_incarnation_of_the_key() {
    let limiter = RateLimiter::with_capacity(4);
    let first = limiter.begin(&TEST, "a").unwrap();
    let second = limiter.begin(&TEST, "a").unwrap();
    // The success of `first` keeps the entry (one attempt still runs); force a new
    // incarnation by removing it directly, as an eviction or prune would.
    first.succeeded();
    limiter.lock().get_mut(TEST.prefix).unwrap().remove("a");
    let held: Vec<_> = (0..3).map(|_| limiter.begin(&TEST, "a").unwrap()).collect();
    // The stale reservation ends: it must not free a slot of the new entry.
    drop(second);
    assert_eq!(limiter.begin(&TEST, "a").unwrap_err(), BUSY_RETRY);
    drop(held);
}
