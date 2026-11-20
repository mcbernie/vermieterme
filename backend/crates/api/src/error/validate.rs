//! Input validation before storing: text is trimmed (see the `vermieterme_db` crate
//! documentation, "Rules the schema cannot enforce") and checked against the column
//! limits of the schema, so a too long value is a 400 with a German message instead of
//! a late database constraint error.
//!
//! A [`Validator`] collects all field errors of a request and turns them into one
//! `400 validation` response with `fields`.

use super::ApiError;

/// `account.name`, `app_user.name` (characters).
pub const NAME_MAX: usize = 255;
/// `app_user.email` (characters, RFC 5321 path limit).
pub const EMAIL_MAX: usize = 254;
/// Password rule of the contract: at least 10 characters.
pub const PASSWORD_MIN: usize = 10;
/// Password rule of the contract: at most 256 characters.
pub const PASSWORD_MAX: usize = 256;

/// Collects field errors of one request.
#[derive(Debug, Default)]
pub struct Validator {
    fields: Vec<(String, String)>,
}

impl Validator {
    /// Empty validator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records an error for `field` (the first error of a field wins).
    pub fn fail(&mut self, field: &str, message: impl Into<String>) {
        if !self.fields.iter().any(|(f, _)| f == field) {
            self.fields.push((field.to_owned(), message.into()));
        }
    }

    /// Required text: trimmed, not empty, at most `max` characters. Returns the trimmed
    /// value (meaningless if an error was recorded).
    pub fn text(&mut self, field: &str, label: &str, value: &str, max: usize) -> String {
        let value = value.trim();
        if value.is_empty() {
            self.fail(field, format!("{label} fehlt"));
        } else if value.chars().count() > max {
            self.fail(
                field,
                format!("{label} ist zu lang (höchstens {max} Zeichen)"),
            );
        }
        value.to_owned()
    }

    /// E-mail address: trimmed, lower-cased, at most [`EMAIL_MAX`] characters, of the
    /// form `local@domain.tld` without spaces.
    pub fn email(&mut self, field: &str, value: &str) -> String {
        let value = value.trim().to_lowercase();
        if !is_email(&value) || value.chars().count() > EMAIL_MAX {
            self.fail(field, "E-Mail-Adresse ist ungültig");
        }
        value
    }

    /// New password: between [`PASSWORD_MIN`] and [`PASSWORD_MAX`] characters. Not
    /// trimmed (spaces are part of a password).
    pub fn password(&mut self, field: &str, value: &str) {
        let len = value.chars().count();
        if len < PASSWORD_MIN {
            self.fail(
                field,
                format!("Passwort muss mindestens {PASSWORD_MIN} Zeichen lang sein"),
            );
        } else if len > PASSWORD_MAX {
            self.fail(
                field,
                format!("Passwort darf höchstens {PASSWORD_MAX} Zeichen lang sein"),
            );
        }
    }

    /// `Ok` if no error was recorded, else `400 validation` with all fields.
    pub fn finish(self) -> Result<(), ApiError> {
        if self.fields.is_empty() {
            Ok(())
        } else {
            Err(ApiError::validation(self.fields))
        }
    }
}

/// Plausibility check of an e-mail address (no full RFC 5322 parser): one `@`, a
/// non-empty local part, a domain with a dot, no whitespace or control characters.
fn is_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !value.chars().any(|c| c.is_whitespace() || c.is_control())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_trimmed_and_limited() {
        let mut v = Validator::new();
        assert_eq!(v.text("name", "Name", "  Max  ", 5), "Max");
        assert!(v.finish().is_ok());

        let mut v = Validator::new();
        v.text("name", "Name", "   ", 5);
        v.text("city", "Ort", "ÄÖÜäöü", 5);
        let err = v.finish().unwrap_err();
        assert_eq!(err.message(), "Name fehlt");
        assert_eq!(err.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[test]
    fn limits_count_characters_not_bytes() {
        let mut v = Validator::new();
        v.text("name", "Name", &"ä".repeat(NAME_MAX), NAME_MAX);
        assert!(v.finish().is_ok());
    }

    #[test]
    fn email_rules() {
        for ok in ["a@b.de", " Max.Muster@Example.ORG "] {
            let mut v = Validator::new();
            v.email("email", ok);
            assert!(v.finish().is_ok(), "{ok}");
        }
        for bad in ["", "a", "a@b", "@b.de", "a@@b.de", "a b@c.de", "a@.de"] {
            let mut v = Validator::new();
            v.email("email", bad);
            assert!(v.finish().is_err(), "{bad}");
        }
        let mut v = Validator::new();
        assert_eq!(v.email("email", " A@B.DE "), "a@b.de");
    }

    #[test]
    fn password_rule() {
        let mut v = Validator::new();
        v.password("password", "123456789");
        assert!(v.finish().unwrap_err().message().contains("mindestens 10"));
        let mut v = Validator::new();
        v.password("password", &"x".repeat(257));
        assert!(v.finish().unwrap_err().message().contains("höchstens 256"));
        let mut v = Validator::new();
        v.password("password", " zehn zeichen ");
        assert!(v.finish().is_ok());
    }
}
