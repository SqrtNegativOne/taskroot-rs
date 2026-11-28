//! Shared Tauri-free atomic JSON persistence for local app data files.

use std::io::{ErrorKind, Write};
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::AppError;

pub const ACCOUNT_FILE: &str = "account.json";
pub const SETTINGS_FILE: &str = "settings.json";
pub const UI_STATE_FILE: &str = "ui-state.json";
pub const STOPWATCH_FOCUS_FILE: &str = "stopwatch-focus.json";
pub const STOPWATCH_HISTORY_FILE: &str = "stopwatch-history.json";

/// All five local JSON files persisted inside the app data directory.
pub const PERSISTED_FILES: [&str; 5] = [
    ACCOUNT_FILE,
    SETTINGS_FILE,
    UI_STATE_FILE,
    STOPWATCH_FOCUS_FILE,
    STOPWATCH_HISTORY_FILE,
];

/// Read a JSON file, returning `None` when it does not exist.
///
/// # Errors
///
/// Returns [`AppError::Store`] when an existing file cannot be read or decoded.
pub fn read_optional_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, AppError> {
    let Some(raw) = read_optional_string(path)? else {
        return Ok(None);
    };
    serde_json::from_str(&raw)
        .map(Some)
        .map_err(|error| AppError::Store(format!("Failed to decode {}: {error}", path.display())))
}

/// Read a JSON file, quarantining a malformed file to `.json.bad` and returning
/// `Ok(None)`.
///
/// # Errors
///
/// Returns [`AppError::Store`] only when an existing file cannot be read.
pub fn read_json_or_quarantine<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, AppError> {
    let Some(raw) = read_optional_string(path)? else {
        return Ok(None);
    };
    match serde_json::from_str(&raw) {
        Ok(value) => Ok(Some(value)),
        Err(error) => {
            eprintln!("Failed to parse {}: {error}", path.display());
            quarantine_bad(path);
            Ok(None)
        }
    }
}

/// Serialize `value` as compact JSON and atomically replace `path`.
///
/// # Errors
///
/// Returns [`AppError::Store`] if serialization or atomic replacement fails.
pub fn write_json_atomic<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<(), AppError> {
    let raw = serde_json::to_vec(value)
        .map_err(|error| AppError::Store(format!("Failed to encode {}: {error}", path.display())))?;
    write_atomic(path, &raw)
}

/// Serialize `value` as pretty-printed JSON and atomically replace `path`.
///
/// # Errors
///
/// Returns [`AppError::Store`] if serialization or atomic replacement fails.
pub fn write_json_pretty_atomic<T: Serialize + ?Sized>(
    path: &Path,
    value: &T,
) -> Result<(), AppError> {
    let raw = serde_json::to_vec_pretty(value)
        .map_err(|error| AppError::Store(format!("Failed to encode {}: {error}", path.display())))?;
    write_atomic(path, &raw)
}

/// Move a corrupted JSON file aside to `<stem>.json.bad`.
pub fn quarantine_bad(path: &Path) {
    let bad = path.with_extension("json.bad");
    let _ = std::fs::remove_file(&bad);
    if let Err(error) = std::fs::rename(path, &bad) {
        eprintln!(
            "Failed to rename {} to {}: {error}",
            path.display(),
            bad.display()
        );
    }
}

fn read_optional_string(path: &Path) -> Result<Option<String>, AppError> {
    match std::fs::read_to_string(path) {
        Ok(raw) => Ok(Some(raw)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(AppError::Store(format!(
            "Failed to read {}: {error}",
            path.display()
        ))),
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|error| {
        AppError::Store(format!(
            "Failed to create temp file for {}: {error}",
            path.display()
        ))
    })?;
    temp.write_all(bytes)
        .and_then(|()| temp.as_file().sync_all())
        .map_err(|error| AppError::Store(format!("Failed to write {}: {error}", path.display())))?;
    temp.persist(path).map_err(|error| {
        AppError::Store(format!(
            "Failed to replace {}: {}",
            path.display(),
            error.error
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::ser;
    use serde_json::Value;

    struct FailingSerialize;

    impl Serialize for FailingSerialize {
        fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
            Err(ser::Error::custom("cannot serialize"))
        }
    }

    fn entries_in(dir: &Path) -> Vec<String> {
        let mut names = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    #[test]
    fn write_json_atomic_replaces_existing_file_without_leaving_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(SETTINGS_FILE);

        write_json_atomic(&path, &serde_json::json!({ "version": 1 })).unwrap();
        write_json_atomic(&path, &serde_json::json!({ "version": 2 })).unwrap();

        assert_eq!(
            read_optional_json::<Value>(&path).unwrap(),
            Some(serde_json::json!({ "version": 2 }))
        );
        assert_eq!(entries_in(dir.path()), vec![SETTINGS_FILE]);
    }

    #[test]
    fn write_json_pretty_atomic_formats_json_and_leaves_no_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(ACCOUNT_FILE);

        write_json_pretty_atomic(&path, &serde_json::json!({ "activeId": "primary" })).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains(r#""activeId": "primary""#));
        assert_eq!(entries_in(dir.path()), vec![ACCOUNT_FILE]);
    }

    #[test]
    fn failed_writes_keep_existing_data_and_clean_up_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(UI_STATE_FILE);
        write_json_atomic(&path, &serde_json::json!({ "kept": true })).unwrap();

        let encode_err = write_json_atomic(&path, &FailingSerialize).unwrap_err();
        assert_eq!(encode_err.code(), "store");
        assert_eq!(
            read_optional_json::<Value>(&path).unwrap(),
            Some(serde_json::json!({ "kept": true }))
        );

        let blocking_dir = dir.path().join("blocked");
        std::fs::create_dir(&blocking_dir).unwrap();
        let persist_err =
            write_json_atomic(&blocking_dir, &serde_json::json!({ "a": 1 })).unwrap_err();
        assert_eq!(persist_err.code(), "store");
        assert_eq!(entries_in(dir.path()), vec!["blocked", UI_STATE_FILE]);
    }

    #[test]
    fn read_json_or_quarantine_moves_malformed_file_aside_and_replaces_prior_bad() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STOPWATCH_HISTORY_FILE);
        let bad = dir.path().join("stopwatch-history.json.bad");
        std::fs::write(&bad, "stale bad").unwrap();
        std::fs::write(&path, "{broken").unwrap();

        let loaded = read_json_or_quarantine::<Value>(&path).unwrap();

        assert_eq!(loaded, None);
        assert!(!path.exists());
        assert_eq!(std::fs::read_to_string(&bad).unwrap(), "{broken");
    }
}
