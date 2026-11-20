//! Password hashing: argon2id for new hashes; legacy bcrypt hashes (Prisma era) are still
//! verified and replaced by argon2id after the next successful sign-in (plan "Auth").
//!
//! Hashing is CPU-heavy, so it runs on the blocking thread pool. Verification against a
//! missing hash still does the full work on a dummy hash, so "unknown user" and "wrong
//! password" cost the same time (security baseline).
//!
//! Every argon2 computation needs a permit of the [`Hasher`] (`PASSWORD_HASH_CONCURRENCY`,
//! default 4): one computation holds about 19 MiB, and without a bound a burst of public
//! requests could exhaust the memory of a small server. Requests wait for a permit at
//! most [`PERMIT_WAIT`] and are then answered `429` (generic, with `Retry-After`).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use argon2::{Argon2, PasswordHasher as _, PasswordVerifier as _};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use vermieterme_db::PasswordHash;

use crate::error::ApiError;

/// Longest wait for a permit before the request is refused.
pub const PERMIT_WAIT: Duration = Duration::from_secs(5);

/// `Retry-After` of a request refused for want of a permit.
const BUSY_RETRY: Duration = Duration::from_secs(1);

/// PHC prefix of a hash with the current algorithm and parameters (argon2id, version 19,
/// OWASP defaults of the `argon2` crate). Other hashes are upgraded after sign-in.
const CURRENT_PREFIX: &str = "$argon2id$v=19$m=19456,t=2,p=1$";

/// Result of a verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verification {
    /// The password matches; `rehash` = the stored hash should be replaced (bcrypt or
    /// outdated argon2 parameters).
    Valid {
        /// Replace the stored hash with [`hash`].
        rehash: bool,
    },
    /// No match, no hash or unreadable hash.
    Invalid,
}

/// Bounded password hashing (shared in `AppState`; clones share the permits).
#[derive(Debug, Clone)]
pub struct Hasher {
    permits: Arc<Semaphore>,
    wait: Duration,
    computed: Arc<AtomicU64>,
}

impl Hasher {
    /// A hasher allowing `concurrency` (at least one) computations at a time.
    pub fn new(concurrency: usize) -> Self {
        Self::with_wait(concurrency, PERMIT_WAIT)
    }

    /// Like [`Hasher::new`] with a custom wait for a permit (tests).
    pub fn with_wait(concurrency: usize, wait: Duration) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(concurrency.max(1))),
            wait,
            computed: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Waits for a permit; after the wait limit `429` (generic). Callers hold **no pool
    /// connection and no transaction** while waiting or computing (here and in
    /// [`Hasher::hash`]/[`Hasher::verify`]): the pool is small, and a login flood would
    /// otherwise block every other request (#102).
    pub async fn permit(&self) -> Result<HashPermit, ApiError> {
        match tokio::time::timeout(self.wait, self.permits.clone().acquire_owned()).await {
            Ok(Ok(permit)) => Ok(HashPermit {
                permit,
                computed: self.computed.clone(),
            }),
            // Closed never happens (the semaphore is never closed); treated like busy.
            Ok(Err(_)) | Err(_) => {
                tracing::warn!("Passwort-Berechnungen ausgelastet, Anfrage abgewiesen");
                Err(ApiError::rate_limited(BUSY_RETRY))
            }
        }
    }

    /// Hashes a password with argon2id and a random salt (waits for a permit).
    pub async fn hash(&self, password: String) -> Result<PasswordHash, ApiError> {
        self.permit().await?.hash(password).await
    }

    /// Verifies `password` against `stored` (waits for a permit); see [`HashPermit::verify`].
    pub async fn verify(
        &self,
        password: String,
        stored: Option<PasswordHash>,
    ) -> Result<Verification, ApiError> {
        self.permit().await?.verify(password, stored).await
    }

    /// Number of argon2/bcrypt computations so far (tests and diagnostics).
    pub fn computed(&self) -> u64 {
        self.computed.load(Ordering::Relaxed)
    }
}

/// Permission for one computation; it is held until the computation ends, even if the
/// request is cancelled meanwhile (the permit moves into the blocking task).
#[derive(Debug)]
pub struct HashPermit {
    permit: OwnedSemaphorePermit,
    computed: Arc<AtomicU64>,
}

impl HashPermit {
    /// Hashes a password with argon2id and a random salt.
    pub async fn hash(self, password: String) -> Result<PasswordHash, ApiError> {
        self.run(move || hash_sync(&password))
            .await?
            .ok_or_else(ApiError::internal)
    }

    /// Verifies `password` against `stored`; without a stored hash a dummy hash is
    /// checked (same cost) and the result is always [`Verification::Invalid`].
    pub async fn verify(
        self,
        password: String,
        stored: Option<PasswordHash>,
    ) -> Result<Verification, ApiError> {
        self.run(move || match stored {
            Some(stored) => verify_sync(&password, stored.as_str()),
            None => {
                let _ = verify_sync(&password, dummy_hash());
                Verification::Invalid
            }
        })
        .await
    }

    /// Runs `work` on the blocking pool while holding the permit.
    async fn run<T: Send + 'static>(
        self,
        work: impl FnOnce() -> T + Send + 'static,
    ) -> Result<T, ApiError> {
        self.computed.fetch_add(1, Ordering::Relaxed);
        let permit = self.permit;
        blocking(move || {
            let _permit = permit;
            work()
        })
        .await
    }
}

/// Runs CPU-heavy work on the blocking pool; a panicked task becomes a 500.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(work).await.map_err(|_| {
        tracing::error!("Passwort-Berechnung abgebrochen");
        ApiError::internal()
    })
}

/// Synchronous argon2id hash; `None` only if the OS random generator fails.
fn hash_sync(password: &str) -> Option<PasswordHash> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .ok()
        .map(|h| PasswordHash::new(h.to_string()))
}

/// Synchronous verification of argon2 (any variant) or bcrypt (`$2a$`, `$2b$`, `$2y$`).
fn verify_sync(password: &str, stored: &str) -> Verification {
    let valid = if stored.starts_with("$argon2") {
        Argon2::default()
            .verify_password(password.as_bytes(), stored)
            .is_ok()
    } else if stored.starts_with("$2") {
        bcrypt::verify(password, stored).unwrap_or(false)
    } else {
        false
    };
    if valid {
        Verification::Valid {
            rehash: !stored.starts_with(CURRENT_PREFIX),
        }
    } else {
        Verification::Invalid
    }
}

/// A valid argon2id hash of a random password, computed once, for equal-cost checks.
fn dummy_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| {
        let random = vermieterme_db::SecretToken::generate();
        hash_sync(random.expose())
            .map(|h| h.as_str().to_owned())
            .unwrap_or_else(|| CURRENT_PREFIX.to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hasher of the tests.
    fn hasher() -> Hasher {
        Hasher::new(2)
    }

    async fn hash(password: &str) -> Result<PasswordHash, ApiError> {
        hasher().hash(password.to_owned()).await
    }

    async fn verify(
        password: &str,
        stored: Option<PasswordHash>,
    ) -> Result<Verification, ApiError> {
        hasher().verify(password.to_owned(), stored).await
    }

    #[tokio::test]
    async fn argon2id_round_trip() {
        let stored = hash("richtig-richtig").await.unwrap();
        assert!(stored.as_str().starts_with(CURRENT_PREFIX));
        let ok = verify("richtig-richtig", Some(stored.clone()))
            .await
            .unwrap();
        assert_eq!(ok, Verification::Valid { rehash: false });
        let bad = verify("falsch-falsch", Some(stored)).await.unwrap();
        assert_eq!(bad, Verification::Invalid);
    }

    #[tokio::test]
    async fn bcrypt_is_verified_and_marked_for_rehash() {
        let legacy = PasswordHash::new(bcrypt::hash("altes-passwort", 4).unwrap());
        let ok = verify("altes-passwort", Some(legacy.clone()))
            .await
            .unwrap();
        assert_eq!(ok, Verification::Valid { rehash: true });
        let bad = verify("anderes-passwort", Some(legacy)).await.unwrap();
        assert_eq!(bad, Verification::Invalid);
    }

    #[tokio::test]
    async fn missing_or_unknown_hash_is_invalid() {
        assert_eq!(verify("x", None).await.unwrap(), Verification::Invalid);
        let plain = PasswordHash::new("klartext");
        assert_eq!(
            verify("klartext", Some(plain)).await.unwrap(),
            Verification::Invalid
        );
    }

    #[tokio::test]
    async fn permits_bound_parallel_computations() {
        let hasher = Hasher::with_wait(1, Duration::from_millis(20));
        let held = hasher.permit().await.unwrap();
        let refused = hasher.permit().await.unwrap_err();
        assert_eq!(refused.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);
        drop(held);
        assert!(hasher.permit().await.is_ok());
    }

    #[tokio::test]
    async fn cancelled_request_keeps_the_permit_until_the_work_ends() {
        let hasher = Hasher::with_wait(1, Duration::from_millis(1));
        let running = tokio::spawn({
            let hasher = hasher.clone();
            async move { hasher.hash("lang-genug-12".to_owned()).await }
        });
        while hasher.computed() == 0 {
            tokio::task::yield_now().await;
        }
        running.abort();
        // The computation still runs on the blocking pool and holds the only permit.
        assert!(hasher.permit().await.is_err());
        let patient = Hasher {
            wait: Duration::from_secs(30),
            ..hasher.clone()
        };
        assert!(patient.permit().await.is_ok());
        assert_eq!(hasher.computed(), 1);
    }
}
