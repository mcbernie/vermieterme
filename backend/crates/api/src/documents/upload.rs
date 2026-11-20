//! `POST /api/documents`: multipart upload (`file`, `billingPeriodId?`, `tenantId?`,
//! `category?`).
//!
//! The file is streamed to a temporary file in the account's directory while its size is
//! counted (413 as soon as it exceeds the limit, before the rest is read) and its type is
//! recognised from the first bytes (400 for anything but PDF, JPEG, PNG, WebP). It is then
//! renamed to its final, generated name and the row is inserted; a [`FileGuard`] removes
//! the file again on every failure.

use axum::extract::multipart::{Field, MultipartError, MultipartRejection};
use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::Response;
use tokio::io::AsyncWriteExt as _;
use vermieterme_db::repo::documents::{self, CATEGORIES, Document, NewDocument};
use vermieterme_db::repo::masterdata::Rejection;
use vermieterme_db::row::new_id;

use super::detached::detached;
use super::file_type::{FileType, HEAD_LEN};
use super::storage::{FileGuard, account_dir, io_failure};
use super::{DocumentJson, MAX_FILE_BYTES, changed_meanwhile, check_assignment};
use crate::auth::AccountCtx;
use crate::error::validate::Validator;
use crate::error::{ApiError, ApiResult};
use crate::masterdata::fields::{ID_INPUT_MAX, filter};
use crate::masterdata::{audited_read_committed, created};
use crate::routes::AppState;

/// Longest accepted text field (ids, category) in bytes; longer is 400.
const TEXT_FIELD_MAX: usize = 1024;
/// Longest stored original name (characters, schema limit).
const NAME_MAX: usize = 255;
/// Name used when the client sends none.
const NAME_FALLBACK: &str = "Dokument";

/// The written file of a request, still guarded.
struct Upload {
    guard: FileGuard,
    file_type: FileType,
    size: i64,
    original_name: String,
}

/// The text fields of a request.
#[derive(Default)]
struct Fields {
    billing_period_id: Option<String>,
    tenant_id: Option<String>,
    category: Option<String>,
}

/// Uploads a document.
pub async fn create(
    State(state): State<AppState>,
    ctx: AccountCtx,
    multipart: Result<Multipart, MultipartRejection>,
) -> ApiResult<Response> {
    let mut multipart =
        multipart.map_err(|_| ApiError::bad_request("Ungültige Anfrage (Formular erwartet)"))?;
    let dir = account_dir(&state.config.upload_dir, ctx.scope.account_id())
        .ok_or_else(ApiError::internal)?;
    let mut upload = None;
    let mut fields = Fields::default();
    while let Some(field) = multipart.next_field().await.map_err(multipart_error)? {
        match field.name() {
            Some("file") if upload.is_some() => {
                return Err(ApiError::field(
                    "file",
                    "Bitte nur eine Datei je Anfrage senden",
                ));
            }
            Some("file") => upload = Some(write_file(&dir, field).await?),
            Some("billingPeriodId") => fields.billing_period_id = Some(text(field).await?),
            Some("tenantId") => fields.tenant_id = Some(text(field).await?),
            Some("category") => fields.category = Some(text(field).await?),
            _ => {}
        }
    }
    let upload = upload.ok_or_else(|| ApiError::field("file", "Keine Datei hochgeladen"))?;
    let new = validate(&fields, &upload)?;
    check_assignment(
        &state,
        &ctx,
        new.billing_period_id.as_deref(),
        new.tenant_id.as_deref(),
    )
    .await?;
    // Insert, commit and `keep` run detached from the request (#121), so a disconnect
    // cannot drop the guard after the commit. Locks in the order of `repo::billing`,
    // hence `READ COMMITTED` (#116).
    let document = detached(async move {
        let document = audited_read_committed(
            &state,
            &ctx,
            ("document.create", "document"),
            "Zuordnung nicht gefunden",
            |d: &Document| d.id.clone(),
            async |tx| {
                let document = documents::create(tx, &ctx.scope, &new).await?;
                Ok(Ok::<_, Rejection>(document))
            },
        )
        .await
        .map_err(changed_meanwhile)?;
        upload.guard.keep();
        Ok(document)
    })
    .await?;
    Ok(created(DocumentJson::from(document)))
}

/// Validated row values of the request.
fn validate(fields: &Fields, upload: &Upload) -> ApiResult<NewDocument> {
    let mut v = Validator::new();
    let id = |v: &mut Validator, field: &str, label: &str, value: &Option<String>| {
        v.opt_text(field, label, filter(value.as_deref()), ID_INPUT_MAX)
    };
    let billing_period_id = id(
        &mut v,
        "billingPeriodId",
        "Abrechnungszeitraum",
        &fields.billing_period_id,
    );
    let tenant_id = id(&mut v, "tenantId", "Mieter", &fields.tenant_id);
    let category = match filter(fields.category.as_deref()) {
        Some(value) => v.code("category", "Kategorie", value, &CATEGORIES),
        None => "other".to_owned(),
    };
    v.finish()?;
    let file_name = upload
        .guard
        .path()
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(ApiError::internal)?
        .to_owned();
    Ok(NewDocument {
        file_name,
        original_name: upload.original_name.clone(),
        mime_type: upload.file_type.mime().to_owned(),
        size_bytes: upload.size,
        category,
        billing_period_id,
        tenant_id,
    })
}

/// Streams the `file` field into the account's directory: temporary name first (starts
/// with a dot, so it can never collide with or be served as a stored file), size and type
/// checked while reading, then renamed to `{uuid}.{ext}`.
async fn write_file(dir: &std::path::Path, mut field: Field<'_>) -> ApiResult<Upload> {
    let original_name = display_name(field.file_name());
    let mut builder = tokio::fs::DirBuilder::new();
    builder.recursive(true);
    // Only the server reads the account's files (they are 0600, the folder 0700).
    #[cfg(unix)]
    builder.mode(0o700);
    builder
        .create(dir)
        .await
        .map_err(|e| io_failure(&e, "Upload-Verzeichnis konnte nicht angelegt werden"))?;
    let id = new_id();
    let mut guard = FileGuard::new(dir.join(format!(".{id}.part")));
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(guard.path())
        .await
        .map_err(|e| io_failure(&e, "Upload-Datei konnte nicht angelegt werden"))?;
    let (mut head, mut size, mut file_type) = (Vec::with_capacity(HEAD_LEN), 0usize, None);
    while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
        size += chunk.len();
        if size > MAX_FILE_BYTES {
            return Err(too_large());
        }
        if file_type.is_none() {
            let missing = HEAD_LEN.saturating_sub(head.len()).min(chunk.len());
            head.extend_from_slice(&chunk[..missing]);
            if head.len() >= HEAD_LEN {
                file_type = Some(detect(&head)?);
            }
        }
        file.write_all(&chunk)
            .await
            .map_err(|e| io_failure(&e, "Upload-Datei konnte nicht geschrieben werden"))?;
    }
    let file_type = match file_type {
        Some(t) => t,
        None => detect(&head)?,
    };
    file.sync_all()
        .await
        .map_err(|e| io_failure(&e, "Upload-Datei konnte nicht geschrieben werden"))?;
    drop(file);
    guard
        .rename(dir.join(format!("{id}.{}", file_type.extension())))
        .await?;
    Ok(Upload {
        guard,
        file_type,
        size: i64::try_from(size).map_err(|_| too_large())?,
        original_name,
    })
}

/// The type of the content, or 400.
fn detect(head: &[u8]) -> ApiResult<FileType> {
    FileType::detect(head).ok_or_else(|| {
        ApiError::field(
            "file",
            "Dateityp nicht erlaubt. Erlaubt: PDF, JPEG, PNG, WebP",
        )
    })
}

/// Reads a short text field (at most [`TEXT_FIELD_MAX`] bytes, else 400).
async fn text(mut field: Field<'_>) -> ApiResult<String> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
        if bytes.len() + chunk.len() > TEXT_FIELD_MAX {
            return Err(ApiError::bad_request("Formularfeld zu lang"));
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|_| ApiError::bad_request("Formularfeld ist kein Text"))
}

/// The uploaded name as shown to users: last path segment only, without control,
/// bidi and zero-width characters, trimmed, at most [`NAME_MAX`] characters; never used
/// as a path.
pub fn display_name(sent: Option<&str>) -> String {
    let last = sent
        .unwrap_or_default()
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default();
    let clean: String = last.chars().filter(|&c| !is_hidden(c)).collect();
    let trimmed: String = clean.trim().chars().take(NAME_MAX).collect();
    let trimmed = trimmed.trim();
    if trimmed.is_empty() {
        NAME_FALLBACK.to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// Control characters (Cc) and the invisible format characters that can disguise a
/// name: zero width and directional marks (U+200B–U+200F), embeddings and overrides
/// (U+202A–U+202E, `"rechnung\u{202E}fdp.exe"` shows as "rechnungexe.pdf"), isolates
/// (U+2066–U+2069) and the byte order mark (U+FEFF).
fn is_hidden(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
}

/// 413 `too_large` (contract text).
fn too_large() -> ApiError {
    ApiError::too_large()
}

/// A broken or oversized multipart body: 413 over the body limit, else 400.
fn multipart_error(error: MultipartError) -> ApiError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        too_large()
    } else {
        ApiError::bad_request("Ungültige Anfrage (Formular unvollständig)")
    }
}

#[cfg(test)]
mod tests {
    use super::display_name;

    #[test]
    fn display_names_are_last_segments_without_controls() {
        assert_eq!(display_name(Some("Rechnung März.pdf")), "Rechnung März.pdf");
        assert_eq!(display_name(Some("../../etc/passwd")), "passwd");
        assert_eq!(display_name(Some("C:\\Users\\x\\a.pdf")), "a.pdf");
        assert_eq!(display_name(Some("a\r\nb.pdf")), "ab.pdf");
        assert_eq!(display_name(Some("  ")), "Dokument");
        assert_eq!(display_name(None), "Dokument");
        assert_eq!(display_name(Some("dir/")), "Dokument");
        assert_eq!(display_name(Some(&"ä".repeat(300))).chars().count(), 255);
        assert_eq!(
            display_name(Some("rechnung\u{202E}fdp.exe")),
            "rechnungfdp.exe"
        );
        assert_eq!(
            display_name(Some(
                "\u{FEFF}a\u{200B}\u{200F}\u{202A}\u{2066}\u{2069}b.pdf"
            )),
            "ab.pdf"
        );
    }
}
