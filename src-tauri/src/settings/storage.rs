//! Store-backed settings and UI-state command bodies.

use crate::error::AppError;
use crate::settings::AppSettings;
use crate::store::{persist, StoreState};
use serde_json::Value;
#[cfg(test)]
use serde_json::Map;

/// # Errors
///
/// This read cannot fail; the signature mirrors the command boundary.
pub(super) async fn read_settings(state: &StoreState) -> Result<AppSettings, AppError> {
    Ok(state.read(|store| store.settings.clone()).await)
}

/// # Errors
///
/// Returns [`AppError::InvalidInput`] for a known key with a wrongly-typed
/// value, or [`AppError::Store`] when the JSON file cannot be written.
pub(super) async fn write_setting(
    state: &StoreState,
    key: &str,
    value: &Value,
) -> Result<(), AppError> {
    let mut store = state.lock().await;
    if !persist::apply_setting(&mut store.settings, key, value)? {
        return Ok(());
    }
    persist::write_settings(&store.data_dir, &store.settings)
}

/// # Errors
///
/// This read cannot fail; the signature mirrors the command boundary.
pub(super) async fn read_ui_state(state: &StoreState, key: &str) -> Result<Value, AppError> {
    Ok(state
        .read(|store| store.ui_state.get(key).cloned().unwrap_or(Value::Null))
        .await)
}

/// # Errors
///
/// Returns [`AppError::Store`] when the UI-state file cannot be written.
pub(super) async fn write_ui_state(
    state: &StoreState,
    key: &str,
    value: &Value,
) -> Result<(), AppError> {
    let mut store = state.lock().await;
    store.ui_state.insert(key.to_string(), value.clone());
    persist::write_ui_state(&store.data_dir, &store.ui_state)
}

/// The whole UI-state map, used by tests and the dev inspector.
#[cfg(test)]
#[must_use]
pub(super) async fn all_ui_state(state: &StoreState) -> Map<String, Value> {
    state.read(|store| store.ui_state.clone()).await
}
