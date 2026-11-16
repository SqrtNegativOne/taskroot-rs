//! Tauri adapter: resolve the managed stopwatch manager from an `AppHandle`.

use tauri::Manager;

use crate::error::AppError;

use super::state::StopwatchManager;

pub fn with_manager<T>(
    app: &tauri::AppHandle,
    apply: impl FnOnce(&StopwatchManager) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let manager = app
        .try_state::<StopwatchManager>()
        .ok_or_else(|| AppError::Internal("Stopwatch state not initialized yet".to_string()))?;
    apply(manager.inner())
}
