//! IPC commands for `settings.json` and `ui-state.json`.
//!
//! The behaviour lives in `settings::storage`; these handlers only resolve the
//! store.

use serde_json::Value;

use crate::ipc::session::store_state;
use crate::error::AppError;
use crate::settings::{metadata, storage, AppSettings};

/// The settings screen's tab/section/setting metadata, derived from the
/// `#[setting(..)]` attributes on [`AppSettings`].
#[tauri::command]
#[must_use]
pub fn get_settings_schema() -> Value {
    metadata::settings_schema_value()
}

/// # Errors
///
/// Returns an error when the store is not initialized or `settings.json` cannot
/// be read.
#[tauri::command]
pub async fn get_settings(app: tauri::AppHandle) -> Result<AppSettings, AppError> {
    let state = store_state(&app)?;
    storage::read_settings(&state).await
}

/// # Errors
///
/// Returns [`AppError::InvalidInput`] for a known setting whose value does not
/// match the field type, and a store error when `settings.json` cannot be
/// written.
#[tauri::command]
pub async fn update_setting(
    app: tauri::AppHandle,
    key: String,
    value: Value,
) -> Result<(), AppError> {
    let state = store_state(&app)?;
    storage::write_setting(&state, &key, &value).await
}

/// Read per-component UI state (filters, sorts, view modes, pane sizes).
/// Returns `null` for an unknown key.
///
/// # Errors
///
/// Returns an error when the store is not initialized or `ui-state.json` cannot
/// be read.
#[tauri::command]
pub async fn get_ui_state(app: tauri::AppHandle, key: String) -> Result<Value, AppError> {
    let state = store_state(&app)?;
    storage::read_ui_state(&state, &key).await
}

/// Persist per-component UI state. See [`get_ui_state`].
///
/// # Errors
///
/// Returns an error when the store is not initialized or `ui-state.json` cannot
/// be written.
#[tauri::command]
pub async fn set_ui_state(
    app: tauri::AppHandle,
    key: String,
    value: Value,
) -> Result<(), AppError> {
    let state = store_state(&app)?;
    storage::write_ui_state(&state, &key, &value).await
}
