//! JSON persistence for `settings.json` and `ui-state.json`.

use crate::error::AppError;
use crate::settings::AppSettings;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{Map, Value};
use std::io::ErrorKind;
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
            eprintln!("Failed to parse {}: {error}; starting from defaults", path.display());
            mark_bad(&path);
            AppSettings::default()
        }),
        Err(error) if error.kind() == ErrorKind::NotFound => AppSettings::default(),
        Err(error) => {
            eprintln!("Failed to read {}: {error}; starting from defaults", path.display());
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
        return Err(AppError::Store("Settings are not a JSON object".to_string()));
    };
    if !map.contains_key(key) {
        return Ok(false);
    }
    let mut probe = Map::new();
    probe.insert(key.to_string(), value.clone());
    serde_json::from_value::<AppSettings>(Value::Object(probe)).map_err(|_| {
        AppError::InvalidInput(format!("Setting '{key}' does not accept the provided value"))
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
/// Returns an error if the value cannot be serialized or written.
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), AppError> {
    let raw = serde_json::to_string(value)
        .map_err(|e| AppError::Store(format!("Failed to encode {}: {e}", path.display())))?;
    std::fs::write(path, raw)
        .map_err(|e| AppError::Store(format!("Failed to write {}: {e}", path.display())))
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
        eprintln!("Failed to rename {} to {}: {error}", path.display(), bad.display());
    }
}
