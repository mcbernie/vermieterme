//! Tests of the configuration parser.

use std::collections::HashMap;

use super::*;

/// A lookup over fixed pairs.
fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    move |name| map.get(name).cloned()
}

const SECRET: &str = "0123456789abcdef0123456789abcdef";

#[test]
fn defaults_apply() {
    let config = Config::from_lookup(lookup(&[("AUTH_SECRET", SECRET)])).unwrap();
    assert_eq!(
        config.database_url,
        "sqlite://data/vermieterme-v2.db?mode=rwc"
    );
    assert_eq!(config.port, 3000);
    assert!(config.cookie_secure);
    assert_eq!(config.session_idle, Duration::from_secs(168 * 3600));
    assert_eq!(config.session_max, Duration::from_secs(30 * 24 * 3600));
    assert!(config.trusted_proxies.is_empty());
    assert_eq!(config.log_format, LogFormat::Json);
    assert_eq!(config.admin_email, None);
}

#[test]
fn auth_secret_is_required_and_long() {
    assert!(matches!(
        Config::from_lookup(lookup(&[])),
        Err(ConfigError::Missing("AUTH_SECRET"))
    ));
    assert!(matches!(
        Config::from_lookup(lookup(&[("AUTH_SECRET", "kurz")])),
        Err(ConfigError::Invalid("AUTH_SECRET", _))
    ));
}

#[test]
fn values_override_defaults() {
    let config = Config::from_lookup(lookup(&[
        ("AUTH_SECRET", SECRET),
        ("COOKIE_SECURE", "false"),
        ("PORT", "8080"),
        ("TRUSTED_PROXIES", "10.0.0.0/8, 127.0.0.1"),
        ("LOG_FORMAT", "text"),
        ("ADMIN_EMAIL", " admin@example.org "),
        ("ADMIN_PASSWORD", "geheim-geheim"),
    ]))
    .unwrap();
    assert!(!config.cookie_secure);
    assert_eq!(config.port, 8080);
    assert_eq!(config.trusted_proxies.len(), 2);
    assert_eq!(config.log_format, LogFormat::Text);
    assert_eq!(config.admin_email.as_deref(), Some("admin@example.org"));
    assert!(!format!("{config:?}").contains("geheim"));
    assert!(!format!("{config:?}").contains(SECRET));
}

#[test]
fn invalid_values_name_the_variable() {
    for (name, bad) in [
        ("PORT", "x"),
        ("COOKIE_SECURE", "ja"),
        ("SESSION_MAX_DAYS", "0"),
        ("TRUSTED_PROXIES", "irgendwo"),
        ("LOG_FORMAT", "xml"),
    ] {
        let err = Config::from_lookup(lookup(&[("AUTH_SECRET", SECRET), (name, bad)])).unwrap_err();
        assert!(err.to_string().starts_with(name), "{err}");
        assert!(!err.to_string().contains(bad) || bad == "0", "{err}");
    }
}

#[test]
fn reference_has_unique_documented_names() {
    let mut names: Vec<_> = VARIABLES.iter().map(|v| v.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), VARIABLES.len());
    assert!(VARIABLES.iter().all(|v| !v.description.is_empty()));
}

#[test]
fn trusted_proxies_with_oversized_prefix_stop_the_start() {
    for bad in [
        "10.0.0.0/33",
        "::/129",
        "::ffff:10.0.0.0/8",
        "::ffff:10.0.0.0/129",
    ] {
        let err = Config::from_lookup(lookup(&[("AUTH_SECRET", SECRET), ("TRUSTED_PROXIES", bad)]))
            .unwrap_err();
        assert!(
            matches!(err, ConfigError::Invalid("TRUSTED_PROXIES", _)),
            "{bad}"
        );
    }
    let config = Config::from_lookup(lookup(&[
        ("AUTH_SECRET", SECRET),
        ("TRUSTED_PROXIES", "::ffff:10.0.0.0/104"),
    ]))
    .unwrap();
    assert!(config.trusted_proxies[0].contains("10.9.8.7".parse().unwrap()));
    assert!(!config.trusted_proxies[0].contains("8.8.8.8".parse().unwrap()));
}

#[test]
fn password_hash_concurrency_is_bounded_and_positive() {
    let config = Config::from_lookup(lookup(&[("AUTH_SECRET", SECRET)])).unwrap();
    assert_eq!(config.password_hash_concurrency, 4);
    let err = Config::from_lookup(lookup(&[
        ("AUTH_SECRET", SECRET),
        ("PASSWORD_HASH_CONCURRENCY", "0"),
    ]))
    .unwrap_err();
    assert!(matches!(
        err,
        ConfigError::Invalid("PASSWORD_HASH_CONCURRENCY", _)
    ));
}

#[test]
fn quota_and_request_timeout_have_defaults_and_must_be_positive() {
    let config = Config::from_lookup(lookup(&[("AUTH_SECRET", SECRET)])).unwrap();
    assert_eq!(config.upload_quota_bytes, 1024 * 1024 * 1024);
    assert_eq!(config.request_timeout, Duration::from_secs(30));
    let set = Config::from_lookup(lookup(&[
        ("AUTH_SECRET", SECRET),
        ("UPLOAD_QUOTA_BYTES", "100000"),
        ("REQUEST_TIMEOUT_SECS", "5"),
    ]))
    .unwrap();
    assert_eq!(set.upload_quota_bytes, 100_000);
    assert_eq!(set.request_timeout, Duration::from_secs(5));
    for name in ["UPLOAD_QUOTA_BYTES", "REQUEST_TIMEOUT_SECS"] {
        let err = Config::from_lookup(lookup(&[("AUTH_SECRET", SECRET), (name, "0")])).unwrap_err();
        assert!(err.to_string().starts_with(name), "{err}");
    }
    let huge = u64::MAX.to_string();
    let err = Config::from_lookup(lookup(&[
        ("AUTH_SECRET", SECRET),
        ("UPLOAD_QUOTA_BYTES", &huge),
    ]))
    .unwrap_err();
    assert!(err.to_string().starts_with("UPLOAD_QUOTA_BYTES"), "{err}");
}
