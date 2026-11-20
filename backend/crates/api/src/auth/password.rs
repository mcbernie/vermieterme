//! Password hashing: argon2id for new hashes; legacy bcrypt hashes (Prisma era) are still
//! verified and replaced by argon2id after the next successful sign-in (plan "Auth").
//!
//! Hashing is CPU-heavy, so it runs on the blocking thread pool. Verification against a
//! missing hash still does the full work on a dummy hash, so "unknown user" and "wrong
//! password" cost the same time (security baseline).

use std::sync::OnceLock;

use argon2::{Argon2, PasswordHasher as _, PasswordVerifier as _};
use vermieterme_db::PasswordHash;

use crate::error::ApiError;

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

/// Hashes a password with argon2id and a random salt.
pub async fn hash(password: String) -> Result<PasswordHash, ApiError> {
    blocking(move || hash_sync(&password))
        .await?
        .ok_or_else(ApiError::internal)
}

/// Verifies `password` against `stored`; without a stored hash a dummy hash is checked
/// (same cost) and the result is always [`Verification::Invalid`].
pub async fn verify(
    password: String,
    stored: Option<PasswordHash>,
) -> Result<Verification, ApiError> {
    blocking(move || match stored {
        Some(stored) => verify_sync(&password, stored.as_str()),
        None => {
            let _ = verify_sync(&password, dummy_hash());
            Verification::Invalid
        }
    })
    .await
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

    #[tokio::test]
    async fn argon2id_round_trip() {
        let stored = hash("richtig-richtig".into()).await.unwrap();
        assert!(stored.as_str().starts_with(CURRENT_PREFIX));
        let ok = verify("richtig-richtig".into(), Some(stored.clone()))
            .await
            .unwrap();
        assert_eq!(ok, Verification::Valid { rehash: false });
        let bad = verify("falsch-falsch".into(), Some(stored)).await.unwrap();
        assert_eq!(bad, Verification::Invalid);
    }

    #[tokio::test]
    async fn bcrypt_is_verified_and_marked_for_rehash() {
        let legacy = PasswordHash::new(bcrypt::hash("altes-passwort", 4).unwrap());
        let ok = verify("altes-passwort".into(), Some(legacy.clone()))
            .await
            .unwrap();
        assert_eq!(ok, Verification::Valid { rehash: true });
        let bad = verify("anderes-passwort".into(), Some(legacy))
            .await
            .unwrap();
        assert_eq!(bad, Verification::Invalid);
    }

    #[tokio::test]
    async fn missing_or_unknown_hash_is_invalid() {
        assert_eq!(
            verify("x".into(), None).await.unwrap(),
            Verification::Invalid
        );
        let plain = PasswordHash::new("klartext");
        assert_eq!(
            verify("klartext".into(), Some(plain)).await.unwrap(),
            Verification::Invalid
        );
    }
}
