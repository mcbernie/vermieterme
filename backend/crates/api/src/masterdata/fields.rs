//! Field rules of the master data requests, as extra methods of the one [`Validator`]
//! (inherent methods can live in any module of the crate): optional text, calendar
//! dates, IBAN, whole-number ranges and code lists. All trim their input.

use vermieterme_domain::CalendarDate;
use vermieterme_domain::iban::{format_iban, is_valid_iban};

use crate::error::validate::Validator;

/// `zip` columns (characters).
pub const ZIP_MAX: usize = 16;
/// Free text columns of 255 characters (street, city, names, bank).
pub const TEXT_MAX: usize = 255;
/// Salutation and phone columns (characters).
pub const SHORT_MAX: usize = 64;

/// Limit for ids in request bodies. Only guards against absurd input: an id longer than
/// the 36 stored characters simply matches nothing and so is 404 like any unknown id.
pub const ID_INPUT_MAX: usize = 256;

/// A list filter: an empty or blank value means "no filter" (as before).
pub fn filter(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|v| !v.is_empty())
}

impl Validator {
    /// Optional text: trimmed, empty or missing -> `None`, at most `max` characters.
    pub fn opt_text(
        &mut self,
        field: &str,
        label: &str,
        value: Option<&str>,
        max: usize,
    ) -> Option<String> {
        let value = value.map(str::trim).filter(|v| !v.is_empty())?;
        if value.chars().count() > max {
            self.fail(
                field,
                format!("{label} ist zu lang (höchstens {max} Zeichen)"),
            );
        }
        Some(value.to_owned())
    }

    /// Optional e-mail address: empty or missing -> `None`, else like [`Validator::email`].
    pub fn opt_email(&mut self, field: &str, value: Option<&str>) -> Option<String> {
        let value = value.map(str::trim).filter(|v| !v.is_empty())?;
        Some(self.email(field, value))
    }

    /// Optional IBAN: empty or missing -> `None`; checked (check digits) and stored in
    /// blocks of four. The value never appears in the message.
    pub fn opt_iban(&mut self, field: &str, value: Option<&str>) -> Option<String> {
        let value = value.map(str::trim).filter(|v| !v.is_empty())?;
        if !is_valid_iban(value) {
            self.fail(field, "IBAN ist ungültig");
        }
        Some(format_iban(value))
    }

    /// Required calendar date `YYYY-MM-DD`; returns the normalised text.
    pub fn date(&mut self, field: &str, label: &str, value: &str) -> String {
        match CalendarDate::parse(value.trim()) {
            Ok(date) => date.to_string(),
            Err(_) => {
                self.fail(field, format!("{label} ist ungültig (JJJJ-MM-TT)"));
                String::new()
            }
        }
    }

    /// Optional calendar date: empty or missing -> `None`.
    pub fn opt_date(&mut self, field: &str, label: &str, value: Option<&str>) -> Option<String> {
        let value = value.map(str::trim).filter(|v| !v.is_empty())?;
        Some(self.date(field, label, value))
    }

    /// Whole number within `min..=max`; returns it unchanged.
    pub fn int_in(&mut self, field: &str, label: &str, value: i64, range: (i64, i64)) -> i64 {
        if value < range.0 || value > range.1 {
            self.fail(
                field,
                format!("{label} muss zwischen {} und {} liegen", range.0, range.1),
            );
        }
        value
    }

    /// Code of a closed list (`kind`, `type`, ...); the list is the one the repository
    /// publishes.
    pub fn code(&mut self, field: &str, label: &str, value: &str, allowed: &[&str]) -> String {
        let value = value.trim();
        if !allowed.contains(&value) {
            self.fail(field, format!("{label} ist ungültig"));
        }
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_text_is_trimmed_and_limited() {
        let mut v = Validator::new();
        assert_eq!(v.opt_text("f", "Feld", Some("  "), 5), None);
        assert_eq!(v.opt_text("f", "Feld", None, 5), None);
        assert_eq!(
            v.opt_text("f", "Feld", Some(" abc "), 5),
            Some("abc".into())
        );
        assert!(v.finish().is_ok());
        let mut v = Validator::new();
        v.opt_text("f", "Feld", Some("abcdef"), 5);
        assert!(v.finish().is_err());
    }

    #[test]
    fn dates_are_checked_and_normalised() {
        let mut v = Validator::new();
        assert_eq!(v.date("d", "Datum", " 2024-02-29 "), "2024-02-29");
        assert_eq!(v.opt_date("d", "Datum", Some("")), None);
        assert!(v.finish().is_ok());
        for bad in ["2023-02-29", "29.02.2024", "2024-13-01", ""] {
            let mut v = Validator::new();
            v.date("d", "Datum", bad);
            assert!(v.finish().is_err(), "{bad}");
        }
    }

    #[test]
    fn iban_is_checked_and_grouped() {
        let mut v = Validator::new();
        let iban = v.opt_iban("iban", Some("de89370400440532013000"));
        assert_eq!(iban.as_deref(), Some("DE89 3704 0044 0532 0130 00"));
        assert!(v.finish().is_ok());
        let mut v = Validator::new();
        v.opt_iban("iban", Some("DE00 1234"));
        let error = v.finish().unwrap_err();
        assert!(!error.message().contains("1234"));
    }

    #[test]
    fn ranges_and_codes() {
        let mut v = Validator::new();
        v.int_in("n", "Zahl", 5, (1, 5));
        v.code("k", "Art", "rent", &["rent", "prepayment"]);
        assert!(v.finish().is_ok());
        let mut v = Validator::new();
        v.int_in("n", "Zahl", 6, (1, 5));
        v.code("k", "Art", "x", &["rent"]);
        assert!(v.finish().is_err());
    }
}
