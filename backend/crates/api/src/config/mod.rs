//! Central configuration: every environment variable with its default and meaning.
//!
//! [`VARIABLES`] is the single source for the configuration reference in
//! `docs/operations.md` (generated from it or checked against it by a test). The values
//! are read through [`Config::from_lookup`], so tests never touch the process
//! environment.

mod cidr;
mod vars;

use std::fmt;
use std::net::IpAddr;
use std::time::Duration;

pub use cidr::Cidr;

pub use vars::{EnvVar, VARIABLES};

/// Minimum length of `AUTH_SECRET` in characters.
pub const AUTH_SECRET_MIN_LEN: usize = 32;
/// Validity of an invitation link for a web user (not configurable).
pub const USER_INVITE_TTL: Duration = Duration::from_secs(7 * 24 * 3600);
/// Validity of a password reset link (decision 22, not configurable).
pub const PASSWORD_RESET_TTL: Duration = Duration::from_secs(24 * 3600);

/// A secret configuration value; `Debug` is redacted.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// The plain value (never log it).
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    /// Redacted.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

/// Log output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// One JSON object per line (default, for log collectors).
    Json,
    /// Human-readable lines.
    Text,
}

/// Validated configuration of the server.
#[derive(Debug, Clone)]
pub struct Config {
    /// `DATABASE_URL`.
    pub database_url: String,
    /// `LEGACY_DATABASE_PATH`.
    pub legacy_database_path: String,
    /// `AUTH_SECRET`.
    pub auth_secret: Secret,
    /// `ADMIN_EMAIL` (bootstrap only).
    pub admin_email: Option<String>,
    /// `ADMIN_PASSWORD` (bootstrap only).
    pub admin_password: Option<Secret>,
    /// `HOST`.
    pub host: IpAddr,
    /// `PORT`.
    pub port: u16,
    /// `STATIC_DIR`.
    pub static_dir: String,
    /// `UPLOAD_DIR`.
    pub upload_dir: String,
    /// `COOKIE_SECURE`.
    pub cookie_secure: bool,
    /// `SESSION_IDLE_HOURS`.
    pub session_idle: Duration,
    /// `SESSION_MAX_DAYS`.
    pub session_max: Duration,
    /// `INVITE_CODE_TTL_DAYS`.
    pub invite_code_ttl: Duration,
    /// `TRUSTED_PROXIES`.
    pub trusted_proxies: Vec<Cidr>,
    /// `PDF_RENDERER_CMD`.
    pub pdf_renderer_cmd: String,
    /// `PDF_RENDER_TIMEOUT_SECS`.
    pub pdf_render_timeout: Duration,
    /// `PDF_RENDER_CONCURRENCY`.
    pub pdf_render_concurrency: usize,
    /// `VPI_API_BASE`.
    pub vpi_api_base: String,
    /// `LOG_LEVEL`.
    pub log_level: String,
    /// `LOG_FORMAT`.
    pub log_format: LogFormat,
}

/// Invalid or missing configuration; names the variable, never its value.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// A required variable is missing or empty.
    #[error("{0} fehlt")]
    Missing(&'static str),
    /// A variable has an invalid value (variable, rule).
    #[error("{0} ist ungültig: {1}")]
    Invalid(&'static str, &'static str),
}

impl Config {
    /// Reads and validates all variables through `lookup` (the process environment in
    /// `main`, a map in tests). Empty values count as unset.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let get = |name: &'static str| value(&lookup, name);
        let auth_secret = get("AUTH_SECRET").ok_or(ConfigError::Missing("AUTH_SECRET"))?;
        if auth_secret.chars().count() < AUTH_SECRET_MIN_LEN {
            return Err(ConfigError::Invalid("AUTH_SECRET", "mindestens 32 Zeichen"));
        }
        Ok(Self {
            database_url: required(&lookup, "DATABASE_URL")?,
            legacy_database_path: required(&lookup, "LEGACY_DATABASE_PATH")?,
            auth_secret: Secret(auth_secret),
            admin_email: get("ADMIN_EMAIL"),
            admin_password: get("ADMIN_PASSWORD").map(Secret),
            host: parse(&lookup, "HOST", "IP-Adresse erwartet")?,
            port: parse(&lookup, "PORT", "Portnummer erwartet")?,
            static_dir: required(&lookup, "STATIC_DIR")?,
            upload_dir: required(&lookup, "UPLOAD_DIR")?,
            cookie_secure: parse(&lookup, "COOKIE_SECURE", "true oder false erwartet")?,
            session_idle: hours(positive(&lookup, "SESSION_IDLE_HOURS")?),
            session_max: hours(positive(&lookup, "SESSION_MAX_DAYS")? * 24),
            invite_code_ttl: hours(positive(&lookup, "INVITE_CODE_TTL_DAYS")? * 24),
            trusted_proxies: cidr::parse_list(&get("TRUSTED_PROXIES").unwrap_or_default()).ok_or(
                ConfigError::Invalid("TRUSTED_PROXIES", "CIDR-Liste erwartet"),
            )?,
            pdf_renderer_cmd: required(&lookup, "PDF_RENDERER_CMD")?,
            pdf_render_timeout: Duration::from_secs(positive(&lookup, "PDF_RENDER_TIMEOUT_SECS")?),
            pdf_render_concurrency: usize::try_from(positive(&lookup, "PDF_RENDER_CONCURRENCY")?)
                .map_err(|_| {
                ConfigError::Invalid("PDF_RENDER_CONCURRENCY", "zu groß")
            })?,
            vpi_api_base: required(&lookup, "VPI_API_BASE")?,
            log_level: required(&lookup, "LOG_LEVEL")?,
            log_format: match required(&lookup, "LOG_FORMAT")?.as_str() {
                "json" => LogFormat::Json,
                "text" => LogFormat::Text,
                _ => {
                    return Err(ConfigError::Invalid(
                        "LOG_FORMAT",
                        "json oder text erwartet",
                    ));
                }
            },
        })
    }
}

/// Value of `name`: the lookup result if non-empty, else the default from [`VARIABLES`].
fn value(lookup: &impl Fn(&str) -> Option<String>, name: &'static str) -> Option<String> {
    lookup(name)
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
        .or_else(|| {
            default_of(name)
                .filter(|d| !d.is_empty())
                .map(str::to_owned)
        })
}

/// Default of a variable from [`VARIABLES`].
fn default_of(name: &str) -> Option<&'static str> {
    VARIABLES
        .iter()
        .find(|v| v.name == name)
        .and_then(|v| v.default)
}

/// A variable that must have a value (all of them have defaults, so only an unknown
/// name can fail here).
fn required(
    lookup: &impl Fn(&str) -> Option<String>,
    name: &'static str,
) -> Result<String, ConfigError> {
    value(lookup, name).ok_or(ConfigError::Missing(name))
}

/// Parses a variable with `FromStr`.
fn parse<T: std::str::FromStr>(
    lookup: &impl Fn(&str) -> Option<String>,
    name: &'static str,
    rule: &'static str,
) -> Result<T, ConfigError> {
    required(lookup, name)?
        .parse()
        .map_err(|_| ConfigError::Invalid(name, rule))
}

/// Parses a positive whole number.
fn positive(
    lookup: &impl Fn(&str) -> Option<String>,
    name: &'static str,
) -> Result<u64, ConfigError> {
    let n: u64 = parse(lookup, name, "positive ganze Zahl erwartet")?;
    if n == 0 {
        return Err(ConfigError::Invalid(name, "positive ganze Zahl erwartet"));
    }
    Ok(n)
}

/// Hours as a duration (saturating).
fn hours(n: u64) -> Duration {
    Duration::from_secs(n.saturating_mul(3600))
}

#[cfg(test)]
mod tests;
