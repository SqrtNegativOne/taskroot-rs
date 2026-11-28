//! Settings and UI state JSON persistence backed by [`crate::storage`].

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::AppError;
use crate::settings::AppSettings;
use crate::storage;

#[must_use]
pub fn settings_path(dir: &Path) -> PathBuf {
    dir.join(storage::SETTINGS_FILE)
}

#[must_use]
pub fn ui_state_path(dir: &Path) -> PathBuf {
    dir.join(storage::UI_STATE_FILE)
}

/// Read `settings.json`. A malformed file is renamed `.bad` and defaults are
/// used; fields are never half-applied.
#[must_use]
pub fn read_settings(dir: &Path) -> AppSettings {
    let path = settings_path(dir);
    match storage::read_json_or_quarantine(&path) {
        Ok(Some(settings)) => settings,
        Ok(None) => AppSettings::default(),
        Err(error) => {
            eprintln!("{error}; starting from defaults");
            AppSettings::default()
        }
    }
}

#[must_use]
pub fn read_ui_state(dir: &Path) -> Map<String, Value> {
    let path = ui_state_path(dir);
    match storage::read_optional_json::<Map<String, Value>>(&path) {
        Ok(Some(map)) => map,
        Ok(None) => Map::new(),
        Err(error) => {
            eprintln!("{error}");
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
    storage::write_json_atomic(&settings_path(dir), settings)
}

/// # Errors
///
/// Returns an error if the UI state cannot be written.
pub fn write_ui_state(dir: &Path, ui_state: &Map<String, Value>) -> Result<(), AppError> {
    storage::write_json_atomic(&ui_state_path(dir), ui_state)
}
