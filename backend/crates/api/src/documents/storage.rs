//! The upload directory: `UPLOAD_DIR/{account_id}/{file_name}`.
//!
//! Paths are only ever built from names that pass [`is_safe_name`]: the account id and
//! the generated file name (UUID v7 plus extension). The uploaded file name is never part
//! of a path. Neither paths nor names are logged (security baseline), only the kind of
//! an I/O error.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::error::{ApiError, ApiResult};

/// `true` for a single path segment of ASCII letters, digits, `-`, `_` and `.` that does
/// not start with a dot (so no `.`, `..`, hidden or temporary names), at most 255 bytes.
pub fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// The directory of an account below `upload_dir`; `None` if the account id is not a safe
/// path segment (never the case for stored ids).
pub fn account_dir(upload_dir: &str, account_id: &str) -> Option<PathBuf> {
    is_safe_name(account_id).then(|| Path::new(upload_dir).join(account_id))
}

/// The path of a stored file; `None` if account id or file name are not safe.
pub fn file_path(upload_dir: &str, account_id: &str, file_name: &str) -> Option<PathBuf> {
    let dir = account_dir(upload_dir, account_id)?;
    is_safe_name(file_name).then(|| dir.join(file_name))
}

/// Logs a failed file operation with its kind only and answers 500.
pub fn io_failure(error: &std::io::Error, what: &'static str) -> ApiError {
    tracing::error!(kind = ?error.kind(), "{what}");
    ApiError::internal()
}

/// Removes the stored files of deleted documents (after the commit, #106). A file that is
/// already gone is fine; other failures are logged with their kind only and do not fail
/// the request: the rows are deleted, a leftover file is harmless and invisible.
pub async fn remove_files(upload_dir: &str, account_id: &str, file_names: &[String]) {
    for name in file_names {
        let Some(path) = file_path(upload_dir, account_id, name) else {
            tracing::warn!(
                "Gespeicherter Dateiname eines Dokuments ist ungültig, Datei bleibt liegen"
            );
            continue;
        };
        match tokio::fs::remove_file(&path).await {
            Err(error) if error.kind() != ErrorKind::NotFound => {
                tracing::warn!(kind = ?error.kind(), "Datei eines Dokuments konnte nicht gelöscht werden");
            }
            _ => {}
        }
    }
}

/// A file on disk that is removed when the guard is dropped, unless [`FileGuard::keep`]
/// was called: covers failed uploads, refused types, a failed insert and a cancelled
/// request (dropped future) alike.
#[derive(Debug)]
pub struct FileGuard {
    path: PathBuf,
    armed: bool,
}

impl FileGuard {
    /// Guards `path`.
    pub fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    /// The guarded path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Moves the file to `target` (same directory, atomic) and guards the new path.
    pub async fn rename(&mut self, target: PathBuf) -> ApiResult<()> {
        tokio::fs::rename(&self.path, &target)
            .await
            .map_err(|e| io_failure(&e, "Hochgeladene Datei konnte nicht abgelegt werden"))?;
        self.path = target;
        Ok(())
    }

    /// Keeps the file (the document row is committed).
    pub fn keep(mut self) {
        self.armed = false;
    }
}

impl Drop for FileGuard {
    /// Removes the file if it was not kept. Synchronous on purpose: `Drop` cannot await,
    /// and a single unlink is short.
    fn drop(&mut self) {
        if self.armed
            && let Err(error) = std::fs::remove_file(&self.path)
            && error.kind() != ErrorKind::NotFound
        {
            tracing::warn!(kind = ?error.kind(), "Unvollständige Upload-Datei konnte nicht gelöscht werden");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{file_path, is_safe_name};

    #[test]
    fn safe_names() {
        for name in ["0197a3b2-7c1d-7e00-8000-000000000000.pdf", "abc", "a_b-c.d"] {
            assert!(is_safe_name(name), "{name}");
        }
        for name in [
            "", ".", "..", ".hidden", "../x", "a/b", "a\\b", "a b", "ä.pdf", "a\0b", "x:y",
        ] {
            assert!(!is_safe_name(name), "{name:?}");
        }
        assert!(!is_safe_name(&"a".repeat(256)));
    }

    #[test]
    fn paths_stay_inside_the_account_directory() {
        let path = file_path("data/uploads", "acc", "f.pdf").unwrap();
        assert_eq!(path, std::path::Path::new("data/uploads/acc/f.pdf"));
        assert!(file_path("data/uploads", "..", "f.pdf").is_none());
        assert!(file_path("data/uploads", "acc", "../f.pdf").is_none());
    }
}
