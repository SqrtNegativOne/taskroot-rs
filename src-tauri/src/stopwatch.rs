#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_possible_truncation)]

use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(
    export,
    export_to = "../../src/lib/bindings/StopwatchState.generated.ts"
)]
pub struct StopwatchState {
    #[ts(type = "number")]
    pub elapsed: u64,
    #[ts(type = "number | null")]
    pub running_since: Option<u64>,
    pub is_break: bool,
    #[ts(type = "number")]
    pub break_allowed_ms: u64,
    #[ts(type = "number | null")]
    pub break_started_at: Option<u64>,
    pub break_sound_played: bool,
}

impl Default for StopwatchState {
    fn default() -> Self {
        Self {
            elapsed: 0,
            running_since: None,
            is_break: false,
            break_allowed_ms: 5 * 60 * 1000,
            break_started_at: None,
            break_sound_played: false,
        }
    }
}

pub struct StopwatchManager(pub Mutex<StopwatchState>);

fn with_locked_state<T>(
    app: &tauri::AppHandle,
    apply: impl FnOnce(&mut StopwatchState) -> T,
) -> Result<T, AppError> {
    let manager = app
        .try_state::<StopwatchManager>()
        .ok_or_else(|| AppError::NotReady("Stopwatch state not initialized yet".to_string()))?;
    let mut guard = manager
        .0
        .lock()
        .map_err(|_| AppError::NotReady("Stopwatch state is unavailable".to_string()))?;
    Ok(apply(&mut guard))
}

fn current_epoch_millis() -> Result<u64, AppError> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| AppError::Internal(format!("System clock error: {e}")))?
        .as_millis() as u64)
}

#[tauri::command]
pub fn get_stopwatch_state(app: tauri::AppHandle) -> Result<StopwatchState, AppError> {
    with_locked_state(&app, |s| s.clone())
}

#[tauri::command]
pub fn toggle_stopwatch(app: tauri::AppHandle) -> Result<StopwatchState, AppError> {
    let now = current_epoch_millis()?;
    let updated = with_locked_state(&app, |s| {
        if let Some(since) = s.running_since {
            s.elapsed += now.saturating_sub(since);
            s.running_since = None;
        } else {
            s.running_since = Some(now);
        }

        s.clone()
    })?;
    let _ = app.emit(crate::events::STOPWATCH_UPDATED, &updated);
    Ok(updated)
}

#[tauri::command]
pub fn reset_stopwatch(app: tauri::AppHandle) -> Result<StopwatchState, AppError> {
    let updated = with_locked_state(&app, |s| {
        s.elapsed = 0;
        s.running_since = None;
        s.clone()
    })?;
    let _ = app.emit(crate::events::STOPWATCH_UPDATED, &updated);
    Ok(updated)
}
