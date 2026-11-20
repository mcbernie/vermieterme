//! The upload directory: `UPLOAD_DIR/{account_id}/{file_name}`.
//!
//! Paths are only ever built from names that pass [`is_safe_name`]: the account id and
//! the generated file name (UUID v7 plus extension). The uploaded file name is never part
//! of a path. Neither paths nor names are logged (security baseline), only the kind of
//! an I/O error.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

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

/// Sets an existing account directory to 0700 (#123): `DirBuilder::mode` only applies
/// when it creates the folder, one made by hand or by an import keeps its rights. Only
/// if the directory belongs to the server's user, told by `own_file` (just created by
/// the server inside it); failures are logged with their kind only, the upload goes on.
pub async fn tighten_dir(dir: &Path, own_file: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let (Ok(meta), Ok(own)) = (
            tokio::fs::metadata(dir).await,
            tokio::fs::metadata(own_file).await,
        ) else {
            return;
        };
        if meta.uid() != own.uid() || meta.mode() & 0o777 == 0o700 {
            return;
        }
        let rights = std::fs::Permissions::from_mode(0o700);
        if let Err(error) = tokio::fs::set_permissions(dir, rights).await {
            tracing::warn!(kind = ?error.kind(), "Rechte des Upload-Verzeichnisses nicht gesetzt");
        }
    }
    #[cfg(not(unix))]
    let _ = (dir, own_file);
}

/// Age after which a temporary upload file counts as left behind by a crash: the time
/// limit of the upload route ([`crate::config::UPLOAD_TIMEOUT`], 5 minutes, decision 45)
/// ends every running upload long before.
pub const STALE_PART_AGE: Duration = Duration::from_secs(3600);
const _: () = assert!(STALE_PART_AGE.as_secs() > crate::config::UPLOAD_TIMEOUT.as_secs());

/// Removes temporary upload files (`.{uuid}.part`, see `upload`) older than `max_age`
/// from the account directories below `upload_dir`: a crash during an upload leaves
/// them, the [`FileGuard`] cannot. Returns how many were removed; failures are logged
/// with their kind only. A missing `upload_dir` (no upload yet) is fine.
pub async fn remove_stale_parts(upload_dir: &Path, max_age: Duration) -> usize {
    let mut removed = 0;
    for dir in entries(upload_dir).await {
        if !dir.file_type().await.is_ok_and(|t| t.is_dir()) {
            continue;
        }
        for entry in entries(&dir.path()).await {
            if is_stale_part(&entry, max_age).await {
                match tokio::fs::remove_file(entry.path()).await {
                    Ok(()) => removed += 1,
                    Err(error) if error.kind() == ErrorKind::NotFound => {}
                    Err(error) => {
                        tracing::warn!(kind = ?error.kind(), "Alte Upload-Datei konnte nicht gelöscht werden");
                    }
                }
            }
        }
    }
    removed
}

/// The entries of a directory; empty (failure logged with its kind) if it cannot be read.
async fn entries(dir: &Path) -> Vec<tokio::fs::DirEntry> {
    let mut found = Vec::new();
    let mut read = match tokio::fs::read_dir(dir).await {
        Ok(read) => read,
        Err(error) if error.kind() == ErrorKind::NotFound => return found,
        Err(error) => {
            tracing::warn!(kind = ?error.kind(), "Upload-Verzeichnis nicht lesbar");
            return found;
        }
    };
    while let Ok(Some(entry)) = read.next_entry().await {
        found.push(entry);
    }
    found
}

/// `true` for a regular file named `.{…}.part` last modified more than `max_age` ago.
async fn is_stale_part(entry: &tokio::fs::DirEntry, max_age: Duration) -> bool {
    let name = entry.file_name();
    let Some(name) = name.to_str() else {
        return false;
    };
    if !(name.starts_with('.') && name.ends_with(".part")) {
        return false;
    }
    let Ok(meta) = entry.metadata().await else {
        return false;
    };
    let age = meta
        .modified()
        .ok()
        .and_then(|m| SystemTime::now().duration_since(m).ok());
    meta.is_file() && age.is_some_and(|age| age > max_age)
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
    use std::time::{Duration, SystemTime};

    use super::{file_path, is_safe_name, remove_stale_parts, tighten_dir};

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

    #[tokio::test]
    async fn only_old_temporary_files_are_removed() {
        let root = std::env::temp_dir().join(format!("vm-parts-{}", vermieterme_db::row::new_id()));
        let dir = root.join("acc");
        std::fs::create_dir_all(&dir).unwrap();
        let old = SystemTime::now() - Duration::from_secs(7200);
        for name in [".old.part", "old.pdf"] {
            let file = std::fs::File::create(dir.join(name)).unwrap();
            file.set_modified(old).unwrap();
        }
        std::fs::File::create(dir.join(".new.part")).unwrap();
        assert_eq!(
            remove_stale_parts(&root, Duration::from_secs(3600)).await,
            1
        );
        let mut left: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        left.sort();
        assert_eq!(left, [".new.part", "old.pdf"]);
        assert_eq!(
            remove_stale_parts(&root.join("fehlt"), Duration::ZERO).await,
            0
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn existing_directory_is_tightened() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("vm-dir-{}", vermieterme_db::row::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        let own = dir.join(".x.part");
        std::fs::File::create(&own).unwrap();
        tighten_dir(&dir, &own).await;
        let mode = std::fs::metadata(&dir).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
