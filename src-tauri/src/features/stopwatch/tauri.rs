//! Tauri adapter: resolve the managed stopwatch state from an `AppHandle`.

use tauri::Manager;

use crate::error::AppError;

use super::state::{StopwatchManager, StopwatchState};

pub fn with_locked_state<T>(
    app: &tauri::AppHandle,
    apply: impl FnOnce(&mut StopwatchState) -> T,
) -> Result<T, AppError> {
    let manager = app
        .try_state::<StopwatchManager>()
        .ok_or_else(|| AppError::Internal("Stopwatch state not initialized yet".to_string()))?;
    let mut guard = manager
        .0
        .lock()
        .map_err(|_| AppError::Internal("Stopwatch state is unavailable".to_string()))?;
    Ok(apply(&mut guard))
}
