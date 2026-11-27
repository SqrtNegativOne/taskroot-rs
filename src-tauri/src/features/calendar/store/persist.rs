//! Atomic JSON persistence for `settings.json`, `ui-state.json` and `account.json`.

use crate::error::AppError;
use crate::settings::AppSettings;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{Map, Value};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

#[must_use]
pub fn settings_path(dir: &Path) -> PathBuf {
    dir.join("settings.json")
}

#[must_use]
pub fn ui_state_path(dir: &Path) -> PathBuf {
    dir.join("ui-state.json")
}

/// Read `settings.json`. A malformed file is renamed `.bad` and defaults are
/// used; fields are never half-applied.
#[must_use]
pub fn read_settings(dir: &Path) -> AppSettings {
    let path = settings_path(dir);
    match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|error| {
            eprintln!(
                "Failed to parse {}: {error}; starting from defaults",
                path.display()
            );
            mark_bad(&path);
            AppSettings::default()
        }),
        Err(error) if error.kind() == ErrorKind::NotFound => AppSettings::default(),
        Err(error) => {
            eprintln!(
                "Failed to read {}: {error}; starting from defaults",
                path.display()
            );
            AppSettings::default()
        }
    }
}

#[must_use]
pub fn read_ui_state(dir: &Path) -> Map<String, Value> {
    let path = ui_state_path(dir);
    match read_optional_json::<Map<String, Value>>(&path) {
        Ok(Some(map)) => map,
        Ok(None) => Map::new(),
        Err(error) => {
            eprintln!("Failed to read {}: {error}", path.display());
            Map::new()
        }
    }
}

/// Validate and apply one setting to the in-memory object.
///
/// Unknown keys are accepted but ignored (the JSON object only stores
/// `AppSettings` fields).
///
/// # Errors
///
/// Returns an error if a known key cannot be deserialized into its field.
pub fn apply_setting(
    settings: &mut AppSettings,
    key: &str,
    value: &Value,
) -> Result<bool, AppError> {
    let Value::Object(mut map) = serde_json::to_value(&*settings)
        .map_err(|e| AppError::Store(format!("Failed to encode settings: {e}")))?
    else {
        return Err(AppError::Store(
            "Settings are not a JSON object".to_string(),
        ));
    };
    if !map.contains_key(key) {
        return Ok(false);
    }
    let mut probe = Map::new();
    probe.insert(key.to_string(), value.clone());
    serde_json::from_value::<AppSettings>(Value::Object(probe)).map_err(|_| {
        AppError::InvalidInput(format!(
            "Setting '{key}' does not accept the provided value"
        ))
    })?;
    map.insert(key.to_string(), value.clone());
    *settings = serde_json::from_value(Value::Object(map))
        .map_err(|e| AppError::Store(format!("Failed to decode settings: {e}")))?;
    Ok(true)
}

/// # Errors
///
/// Returns an error if the settings cannot be written.
pub fn write_settings(dir: &Path, settings: &AppSettings) -> Result<(), AppError> {
    write_json(&settings_path(dir), settings)
}

/// # Errors
///
/// Returns an error if the UI state cannot be written.
pub fn write_ui_state(dir: &Path, ui_state: &Map<String, Value>) -> Result<(), AppError> {
    write_json(&ui_state_path(dir), ui_state)
}

/// # Errors
///
/// Returns an error if the value cannot be serialized or atomically written.
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), AppError> {
    let raw = serde_json::to_string(value)
        .map_err(|e| AppError::Store(format!("Failed to encode {}: {e}", path.display())))?;
    write_atomic(path, raw.as_bytes())
}

/// # Errors
///
/// Returns an error if the value cannot be serialized or atomically written.
pub fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> Result<(), AppError> {
    let raw = serde_json::to_vec_pretty(value)
        .map_err(|e| AppError::Store(format!("Failed to encode {}: {e}", path.display())))?;
    write_atomic(path, &raw)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| {
        AppError::Store(format!(
            "Failed to create temp file for {}: {e}",
            path.display()
        ))
    })?;
    temp.write_all(bytes)
        .and_then(|()| temp.as_file().sync_all())
        .map_err(|e| AppError::Store(format!("Failed to write {}: {e}", path.display())))?;
    temp.persist(path).map_err(|e| {
        AppError::Store(format!(
            "Failed to replace {}: {}",
            path.display(),
            e.error
        ))
    })?;
    Ok(())
}

/// Read a JSON file, returning `None` when it does not exist.
///
/// # Errors
///
/// Returns an error if the file exists but cannot be read or decoded.
pub fn read_optional_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, AppError> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(AppError::Store(format!(
                "Failed to read {}: {error}",
                path.display()
            )));
        }
    };
    serde_json::from_str(&raw)
        .map(Some)
        .map_err(|e| AppError::Store(format!("Failed to decode {}: {e}", path.display())))
}

fn mark_bad(path: &Path) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde::ser;

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
    fn write_json_replaces_existing_file_without_leaving_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = settings_path(dir.path());

        write_json(&path, &serde_json::json!({ "version": 1 })).unwrap();
        write_json(&path, &serde_json::json!({ "version": 2 })).unwrap();

        assert_eq!(
            read_optional_json::<Value>(&path).unwrap(),
            Some(serde_json::json!({ "version": 2 }))
        );
        assert_eq!(entries_in(dir.path()), vec!["settings.json"]);
    }

    #[test]
    fn write_json_pretty_formats_json_and_leaves_no_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("account.json");

        write_json_pretty(&path, &serde_json::json!({ "activeId": "primary" })).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains(r#""activeId": "primary""#));
        assert_eq!(entries_in(dir.path()), vec!["account.json"]);
    }

    #[test]
    fn failed_writes_keep_existing_data_and_clean_up_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = ui_state_path(dir.path());
        write_json(&path, &serde_json::json!({ "kept": true })).unwrap();

        let encode_err = write_json(&path, &FailingSerialize).unwrap_err();
        assert_eq!(encode_err.code(), "store");
        assert_eq!(
            read_optional_json::<Value>(&path).unwrap(),
            Some(serde_json::json!({ "kept": true }))
        );

        let blocking_dir = dir.path().join("blocked");
        std::fs::create_dir(&blocking_dir).unwrap();
        let persist_err = write_json(&blocking_dir, &serde_json::json!({ "a": 1 })).unwrap_err();
        assert_eq!(persist_err.code(), "store");
        assert_eq!(entries_in(dir.path()), vec!["blocked", "ui-state.json"]);
    }
}
