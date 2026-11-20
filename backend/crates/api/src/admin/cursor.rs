//! Opaque paging cursor of the audit log: base64url (no padding) of `"{at}:{id}"`
//! (contract section 4). Clients must not interpret it.

use vermieterme_db::repo::audit::AuditCursor;

/// base64url alphabet (RFC 4648, section 5).
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
/// Longest accepted cursor (an id has at most 36 characters).
const MAX_LEN: usize = 128;

/// Encodes a cursor.
pub fn encode(cursor: &AuditCursor) -> String {
    base64url(format!("{}:{}", cursor.at, cursor.id).as_bytes())
}

/// Decodes a cursor; `None` for anything malformed.
pub fn decode(text: &str) -> Option<AuditCursor> {
    if text.is_empty() || text.len() > MAX_LEN {
        return None;
    }
    let bytes = unbase64url(text)?;
    let plain = String::from_utf8(bytes).ok()?;
    let (at, id) = plain.split_once(':')?;
    let valid_id = !id.is_empty()
        && id.len() <= 36
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
    if !valid_id {
        return None;
    }
    Some(AuditCursor {
        at: at.parse().ok()?,
        id: id.to_owned(),
    })
}

/// base64url without padding.
fn base64url(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..=chunk.len() {
            let index = (n >> (18 - 6 * i)) & 0x3f;
            out.push(char::from(ALPHABET[index as usize]));
        }
    }
    out
}

/// Inverse of [`base64url`]; `None` for invalid characters or lengths.
fn unbase64url(text: &str) -> Option<Vec<u8>> {
    if text.len() % 4 == 1 {
        return None;
    }
    let values = text
        .bytes()
        .map(|c| {
            ALPHABET
                .iter()
                .position(|&a| a == c)
                .and_then(|p| u32::try_from(p).ok())
        })
        .collect::<Option<Vec<u32>>>()?;
    let mut out = Vec::with_capacity(values.len() * 3 / 4);
    for chunk in values.chunks(4) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, v)| n | (v << (18 - 6 * i)));
        for i in 0..chunk.len() - 1 {
            out.push(u8::try_from((n >> (16 - 8 * i)) & 0xff).ok()?);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64url_matches_rfc_4648_vectors() {
        assert_eq!(base64url(b"f"), "Zg");
        assert_eq!(base64url(b"fo"), "Zm8");
        assert_eq!(base64url(b"foo"), "Zm9v");
        assert_eq!(base64url(b"foob"), "Zm9vYg");
        assert_eq!(base64url(&[0xfb, 0xff]), "-_8");
        assert_eq!(unbase64url("Zm9vYg").unwrap(), b"foob");
        assert_eq!(unbase64url("-_8").unwrap(), vec![0xfb, 0xff]);
    }

    #[test]
    fn cursor_round_trip() {
        let cursor = AuditCursor {
            at: 1_769_860_800_123,
            id: "01890a5d-ac96-774b-bcce-b302099a8057".into(),
        };
        assert_eq!(decode(&encode(&cursor)), Some(cursor));
    }

    #[test]
    fn rejects_malformed_cursors() {
        for bad in [
            "",
            "!!!!",
            "Zg",
            "A",
            &base64url(b"x:abc"),
            &base64url(b"1:a b"),
            &"A".repeat(200),
        ] {
            assert_eq!(decode(bad), None, "{bad}");
        }
    }
}
