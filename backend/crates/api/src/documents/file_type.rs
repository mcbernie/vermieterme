//! The allowed document types, recognised by their content (magic bytes), never by the
//! name or the type the client sends (contract section 8: PDF, JPEG, PNG, WebP).
//!
//! Four fixed signatures are checked here directly; a detection library would add more
//! types than are allowed and nothing the check needs.

/// Bytes needed to recognise every allowed type (WebP: `RIFF` + size + `WEBP`).
pub const HEAD_LEN: usize = 12;

/// An allowed document type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// PDF document.
    Pdf,
    /// JPEG image.
    Jpeg,
    /// PNG image.
    Png,
    /// WebP image.
    Webp,
}

impl FileType {
    /// Recognises the type from the first bytes of a file (at least [`HEAD_LEN`] bytes,
    /// fewer only if the file is shorter); `None` for anything not allowed.
    pub fn detect(head: &[u8]) -> Option<Self> {
        if head.starts_with(b"%PDF-") {
            Some(Self::Pdf)
        } else if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some(Self::Jpeg)
        } else if head.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(Self::Png)
        } else if head.len() >= HEAD_LEN && head.starts_with(b"RIFF") && &head[8..12] == b"WEBP" {
            Some(Self::Webp)
        } else {
            None
        }
    }

    /// The content type stored and sent on download.
    pub fn mime(self) -> &'static str {
        match self {
            Self::Pdf => "application/pdf",
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Webp => "image/webp",
        }
    }

    /// File extension of the stored file (for operators browsing the upload directory).
    pub fn extension(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
        }
    }

    /// The type with this stored content type, if it is an allowed one (legacy rows may
    /// carry other values; those are only offered as a download).
    pub fn from_mime(mime: &str) -> Option<Self> {
        [Self::Pdf, Self::Jpeg, Self::Png, Self::Webp]
            .into_iter()
            .find(|t| t.mime() == mime)
    }
}

#[cfg(test)]
mod tests {
    use super::FileType;

    #[test]
    fn detects_the_allowed_types() {
        assert_eq!(FileType::detect(b"%PDF-1.7\n%..."), Some(FileType::Pdf));
        assert_eq!(
            FileType::detect(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0x10]),
            Some(FileType::Jpeg)
        );
        assert_eq!(
            FileType::detect(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR"),
            Some(FileType::Png)
        );
        assert_eq!(
            FileType::detect(b"RIFF\x24\0\0\0WEBPVP8 "),
            Some(FileType::Webp)
        );
    }

    #[test]
    fn rejects_other_content() {
        for head in [
            &b""[..],
            b"<html><script>",
            b"GIF89a......",
            b"PK\x03\x04 zip....",
            b"RIFF\x24\0\0\0WAVEfmt ",
            b"RIFF\x24\0\0\0WEB",
            b" %PDF-1.7",
        ] {
            assert_eq!(FileType::detect(head), None, "{head:?}");
        }
    }

    #[test]
    fn mime_round_trip() {
        for t in [FileType::Pdf, FileType::Jpeg, FileType::Png, FileType::Webp] {
            assert_eq!(FileType::from_mime(t.mime()), Some(t));
        }
        assert_eq!(FileType::from_mime("text/html"), None);
    }
}
