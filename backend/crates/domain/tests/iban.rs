//! Port of `src/__tests__/lib/iban.test.ts` plus edge cases. All IBANs are public
//! example numbers, no personal data.

use vermieterme_domain::iban::{format_iban, is_valid_iban};

#[test]
fn validates_correct_ibans() {
    assert!(is_valid_iban("DE89370400440532013000"));
    assert!(is_valid_iban("DE89 3704 0044 0532 0130 00"));
    assert!(is_valid_iban("de89370400440532013000"));
    assert!(is_valid_iban("AT611904300234573201"));
    assert!(is_valid_iban("CH9300762011623852957"));
    assert!(is_valid_iban("DE02120300000000202051"));
}

#[test]
fn rejects_invalid_input() {
    assert!(!is_valid_iban(""));
    assert!(!is_valid_iban("hello world"));
    // Wrong check digits.
    assert!(!is_valid_iban("DE00370400440532013000"));
    // German IBAN must have 22 characters, this has 21.
    assert!(!is_valid_iban("DE8937040044053201300"));
    assert!(!is_valid_iban("XX89370400440532013000"));
}

#[test]
fn rejects_edge_cases() {
    // Too short, too long, non-ASCII digit, separator characters.
    assert!(!is_valid_iban("DE89370"));
    assert!(!is_valid_iban(&format!("DE89{}", "1".repeat(31))));
    assert!(!is_valid_iban("DE89 3704 0044 0532 0130 0٠"));
    assert!(!is_valid_iban("DE89-3704-0044-0532-0130-00"));
    assert!(is_valid_iban("\tDE89 3704\u{a0}0044 0532 0130 00\n"));
}

#[test]
fn formats_in_groups_of_four() {
    assert_eq!(
        format_iban("DE89370400440532013000"),
        "DE89 3704 0044 0532 0130 00"
    );
    assert_eq!(
        format_iban("DE89 3704 0044 0532 0130 00"),
        "DE89 3704 0044 0532 0130 00"
    );
    assert_eq!(
        format_iban("de89370400440532013000"),
        "DE89 3704 0044 0532 0130 00"
    );
    assert_eq!(format_iban(""), "");
}
