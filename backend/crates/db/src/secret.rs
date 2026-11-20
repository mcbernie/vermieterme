//! Secret tokens (session cookie, invitation and reset links) and their stored hashes.
//!
//! Only [`TokenHash`] (SHA-256, hex) ever reaches the database; the plain token exists in
//! memory and in the response that hands it to the client. `Debug` is redacted.

use std::fmt;

use sha2::{Digest, Sha256};

/// Number of random bytes of a generated token (256 bit).
const TOKEN_BYTES: usize = 32;

/// A secret token in plain text. Never log it, never store it.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretToken(String);

impl SecretToken {
    /// Generates a new token with 256 bit from the operating system's CSPRNG (via the
    /// thread-local generator of `rand`), hex-encoded.
    pub fn generate() -> Self {
        let mut bytes = [0u8; TOKEN_BYTES];
        rand::fill(&mut bytes);
        Self(hex(&bytes))
    }

    /// Wraps a token presented by a client (cookie value, link parameter).
    pub fn from_client(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    /// The plain value, for handing it to the client exactly once.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// SHA-256 of the token; the only form stored in the database.
    pub fn hash(&self) -> TokenHash {
        TokenHash(hex(&Sha256::digest(self.0.as_bytes())))
    }
}

impl fmt::Debug for SecretToken {
    /// Redacted: the value never appears in logs or panics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretToken(***)")
    }
}

/// Hex-encoded SHA-256 of a [`SecretToken`] (64 characters).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenHash(String);

impl TokenHash {
    /// The hex digest as stored in the database.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A password hash (argon2id or legacy bcrypt string). Never logged: `Debug` is redacted.
#[derive(Clone, PartialEq, Eq)]
pub struct PasswordHash(String);

impl PasswordHash {
    /// Wraps a hash produced by the password module of the HTTP layer.
    pub fn new(hash: impl Into<String>) -> Self {
        Self(hash.into())
    }

    /// The stored string, for verification.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PasswordHash {
    /// Redacted.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PasswordHash(***)")
    }
}

/// Lower-case hex encoding.
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from(DIGITS[usize::from(b >> 4)]));
        out.push(char::from(DIGITS[usize::from(b & 0x0f)]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_tokens_are_long_and_distinct() {
        let a = SecretToken::generate();
        let b = SecretToken::generate();
        assert_eq!(a.expose().len(), 64);
        assert!(a.expose().chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    #[test]
    fn hash_is_sha256_hex() {
        // SHA-256("abc"), FIPS 180-2 test vector.
        let h = SecretToken::from_client("abc").hash();
        assert_eq!(
            h.as_str(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn debug_is_redacted() {
        let t = SecretToken::from_client("geheim");
        assert!(!format!("{t:?}").contains("geheim"));
    }
}
