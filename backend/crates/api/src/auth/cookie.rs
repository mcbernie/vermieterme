//! The session cookie `vm_session`: `HttpOnly`, `SameSite=Lax`, path `/`, `Secure` unless
//! `COOKIE_SECURE=false` (local development without TLS only).

use std::time::Duration;

use axum::http::{HeaderMap, HeaderValue, header};
use vermieterme_db::SecretToken;

/// Name of the session cookie (interface list).
pub const SESSION_COOKIE: &str = "vm_session";

/// Length of a session token: 32 random bytes as lower-case hex.
const TOKEN_LEN: usize = 64;

/// The session token from the `Cookie` headers, if present and well-formed. Malformed
/// values are treated as absent (no database lookup for garbage).
pub fn read(headers: &HeaderMap) -> Option<SecretToken> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == SESSION_COOKIE)
        .map(|(_, value)| value.trim())
        .filter(|v| {
            v.len() == TOKEN_LEN && v.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        })
        .map(SecretToken::from_client)
}

/// `Set-Cookie` value that stores `token` for `max_age`.
pub fn set(token: &SecretToken, max_age: Duration, secure: bool) -> HeaderValue {
    build(token.expose(), max_age.as_secs(), secure)
}

/// `Set-Cookie` value that deletes the cookie.
pub fn clear(secure: bool) -> HeaderValue {
    build("", 0, secure)
}

/// Assembles the cookie attributes. The value is hex (or empty), so the header is
/// always valid; the fallback only satisfies the type.
fn build(value: &str, max_age: u64, secure: bool) -> HeaderValue {
    let secure = if secure { "; Secure" } else { "" };
    let cookie = format!(
        "{SESSION_COOKIE}={value}; Path=/; Max-Age={max_age}; HttpOnly; SameSite=Lax{secure}"
    );
    HeaderValue::from_str(&cookie)
        .unwrap_or_else(|_| HeaderValue::from_static("vm_session=; Max-Age=0"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_cookie(value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(header::COOKIE, value.parse().unwrap());
        h
    }

    #[test]
    fn reads_well_formed_token_only() {
        let token = SecretToken::generate();
        let h = with_cookie(&format!("theme=dark; vm_session={}", token.expose()));
        assert_eq!(read(&h), Some(token));
        assert_eq!(read(&with_cookie("vm_session=abc")), None);
        assert_eq!(
            read(&with_cookie(&format!("vm_session={}", "G".repeat(64)))),
            None
        );
        assert_eq!(read(&HeaderMap::new()), None);
    }

    #[test]
    fn cookie_attributes() {
        let token = SecretToken::from_client("ab");
        let set = set(&token, Duration::from_secs(60), true);
        assert_eq!(
            set.to_str().unwrap(),
            "vm_session=ab; Path=/; Max-Age=60; HttpOnly; SameSite=Lax; Secure"
        );
        assert!(!clear(false).to_str().unwrap().contains("Secure"));
        assert!(clear(true).to_str().unwrap().contains("Max-Age=0"));
    }
}
