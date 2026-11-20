//! IBAN validation (ISO 13616, check digits mod 97) and display grouping.
//!
//! IBANs are personal data: callers must never log them or echo them in errors.

/// Expected lengths of the countries checked so far (as in the old `src/lib/iban.ts`).
/// Other country codes are only checked for format and check digits.
const COUNTRY_LENGTHS: [(&str, usize); 31] = [
    ("DE", 22),
    ("AT", 20),
    ("CH", 21),
    ("LI", 21),
    ("LU", 20),
    ("FR", 27),
    ("NL", 18),
    ("BE", 16),
    ("IT", 27),
    ("ES", 24),
    ("PT", 25),
    ("PL", 28),
    ("CZ", 24),
    ("GB", 22),
    ("IE", 22),
    ("DK", 18),
    ("SE", 24),
    ("NO", 15),
    ("FI", 18),
    ("GR", 27),
    ("HU", 28),
    ("SK", 24),
    ("SI", 19),
    ("HR", 21),
    ("RO", 24),
    ("BG", 22),
    ("EE", 20),
    ("LT", 20),
    ("LV", 21),
    ("MT", 31),
    ("CY", 28),
];

/// Removes whitespace and converts ASCII letters to upper case. Other characters stay
/// unchanged, so non-ASCII input (e.g. `ſ`, which Unicode upper-cases to `S`) is
/// rejected by the format check instead of turning into a valid letter.
fn normalize(iban: &str) -> String {
    iban.chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// Whether `iban` is a valid IBAN. Spaces and lower case are accepted; an empty
/// string and any non-ASCII letter or digit are invalid. Country codes without an
/// entry in the length table are not rejected as such, only checked for format and
/// check digits (as before).
pub fn is_valid_iban(iban: &str) -> bool {
    let cleaned = normalize(iban);
    let bytes = cleaned.as_bytes();
    // Two letters, two check digits, 4 to 30 alphanumerics (ASCII only).
    let format_ok = (8..=34).contains(&bytes.len())
        && bytes[..2].iter().all(u8::is_ascii_uppercase)
        && bytes[2..4].iter().all(u8::is_ascii_digit)
        && bytes[4..]
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
    if !format_ok {
        return false;
    }
    let country = &cleaned[..2];
    if let Some((_, length)) = COUNTRY_LENGTHS.iter().find(|(code, _)| *code == country)
        && bytes.len() != *length
    {
        return false;
    }
    // Move the first four characters to the end, letters count as 10..=35.
    let rearranged = bytes[4..].iter().chain(&bytes[..4]);
    let remainder = rearranged.fold(0u32, |acc, &b| {
        if b.is_ascii_digit() {
            (acc * 10 + u32::from(b - b'0')) % 97
        } else {
            (acc * 100 + u32::from(b - b'A' + 10)) % 97
        }
    });
    remainder == 1
}

/// Groups an IBAN in blocks of four characters (`DE89 3704 0044 0532 0130 00`),
/// upper case, regardless of the input spacing.
pub fn format_iban(iban: &str) -> String {
    let cleaned = normalize(iban);
    let mut out = String::with_capacity(cleaned.len() + cleaned.len() / 4);
    for (i, c) in cleaned.chars().enumerate() {
        if i > 0 && i.is_multiple_of(4) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}
