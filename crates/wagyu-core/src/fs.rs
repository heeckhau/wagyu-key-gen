//! Filesystem helpers: sensitive file creation (ported from `sensitive_opener`) and the small
//! directory utilities the UI needs (ported from `src/electron/BashUtils.ts`).

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::error::{Error, Result};

/// True if `path` exists and is a directory.
pub fn does_directory_exist(path: &Path) -> bool {
    path.is_dir()
}

/// True if a file can actually be created in `dir`.
///
/// Checking permission bits is not enough on Windows, so a temporary file is created and removed.
pub fn is_directory_writable(dir: &Path) -> bool {
    dir.is_dir() && tempfile::tempfile_in(dir).is_ok()
}

/// The first regular file in `dir` (in name order) whose name starts with `starts_with`.
pub fn find_first_file(dir: &Path, starts_with: &str) -> Result<Option<PathBuf>> {
    let mut names: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| Error::io("cannot read directory", dir, e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(starts_with))
                .unwrap_or(false)
        })
        .collect();
    names.sort();
    Ok(names.into_iter().next())
}

/// Creates `dir` if it does not exist yet (one level, like `os.mkdir`).
pub fn ensure_directory(dir: &Path) -> Result<()> {
    if !dir.exists() {
        std::fs::create_dir(dir).map_err(|e| Error::io("cannot create directory", dir, e))?;
    }
    Ok(())
}

/// Creates a new file that must not already exist. On Unix the file is created read-only for the
/// owner (`0o400`), matching the deposit-cli.
pub fn create_sensitive_file(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o400);
    }
    options
        .open(path)
        .map_err(|e| Error::io("cannot create file", path, e))
}

/// Serialises `value` as compact JSON into a newly created sensitive file.
pub fn write_sensitive_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut file = create_sensitive_file(path)?;
    let json = serde_json::to_vec(value)?;
    file.write_all(&json)
        .and_then(|_| file.sync_all())
        .map_err(|e| Error::io("cannot write file", path, e))
}

/// Seconds since the Unix epoch, used in output file names like the Python `time.time()`.
pub fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_checks() {
        let dir = tempfile::tempdir().unwrap();
        assert!(does_directory_exist(dir.path()));
        assert!(is_directory_writable(dir.path()));
        assert!(!does_directory_exist(&dir.path().join("missing")));
        assert!(!is_directory_writable(&dir.path().join("missing")));
    }

    #[test]
    fn sensitive_file_is_exclusive_and_read_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keystore-x.json");
        write_sensitive_json(&path, &serde_json::json!({"a": 1})).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), r#"{"a":1}"#);
        assert!(write_sensitive_json(&path, &serde_json::json!({"a": 2})).is_err());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o400
            );
        }
    }

    #[test]
    fn first_file_by_prefix() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("deposit_data-1.json"), "[]").unwrap();
        std::fs::write(dir.path().join("keystore-b.json"), "{}").unwrap();
        std::fs::write(dir.path().join("keystore-a.json"), "{}").unwrap();
        std::fs::create_dir(dir.path().join("keystore-dir")).unwrap();
        assert_eq!(
            find_first_file(dir.path(), "keystore").unwrap(),
            Some(dir.path().join("keystore-a.json"))
        );
        assert_eq!(find_first_file(dir.path(), "bls").unwrap(), None);
    }
}
