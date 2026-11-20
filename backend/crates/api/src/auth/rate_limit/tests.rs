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
    assert_eq!(limiter.lock().len(), 2);
}
