#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_possible_truncation)]

use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use ts_rs::TS;

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
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
    pub break_elapsed: u64,
    #[ts(type = "number | null")]
    pub break_running_since: Option<u64>,
    #[ts(type = "number | null")]
    pub paused_until: Option<u64>,
}

pub struct StopwatchManager(pub Mutex<StopwatchState>);

fn with_locked_state<T>(
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

fn current_epoch_millis() -> Result<u64, AppError> {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| AppError::Internal(format!("System clock error: {e}")))?
        .as_millis();
    u64::try_from(millis).map_err(|_| AppError::Internal("Timestamp too large".to_string()))
}

#[tauri::command]
pub fn get_stopwatch_state(app: tauri::AppHandle) -> Result<StopwatchState, AppError> {
    let now = current_epoch_millis()?;
    with_locked_state(&app, |s| {
        if s.paused_until.is_some_and(|until| until <= now) {
            s.paused_until = None;
        }
        s.clone()
    })
}

/// Toggle a timed pause. An active or expired pause is cleared; otherwise the
/// timer is paused for `pause_minutes`.
#[tauri::command]
pub fn toggle_pause(app: tauri::AppHandle, pause_minutes: u64) -> Result<StopwatchState, AppError> {
    let now = current_epoch_millis()?;
    let updated = with_locked_state(&app, |s| {
        if s.paused_until.is_some_and(|until| until > now) {
            s.paused_until = None;
        } else {
            let duration = pause_minutes.max(1).saturating_mul(60_000);
            s.paused_until = Some(now.saturating_add(duration));
        }
        s.clone()
    })?;
    let _ = app.emit(crate::events::STOPWATCH_UPDATED, &updated);
    Ok(updated)
}

/// Extend (`delta_minutes > 0`) or shrink an active pause by whole minutes. A
/// pause reduced to zero or below unpauses.
#[tauri::command]
pub fn adjust_pause(app: tauri::AppHandle, delta_minutes: i64) -> Result<StopwatchState, AppError> {
    let now = current_epoch_millis()?;
    let updated = with_locked_state(&app, |s| {
        if let Some(until) = s.paused_until {
            let delta = i128::from(delta_minutes).saturating_mul(60_000);
            let target = i128::from(until).saturating_add(delta);
            s.paused_until = if target <= i128::from(now) {
                None
            } else {
                u64::try_from(target).ok()
            };
        }
        s.clone()
    })?;
    let _ = app.emit(crate::events::STOPWATCH_UPDATED, &updated);
    Ok(updated)
}

#[tauri::command]
pub fn toggle_stopwatch(app: tauri::AppHandle) -> Result<StopwatchState, AppError> {
    let now = current_epoch_millis()?;
    let updated = with_locked_state(&app, |s| {
        if s.is_break {
            if let Some(since) = s.break_running_since {
                s.break_elapsed = s.break_elapsed.saturating_add(now.saturating_sub(since));
                s.break_running_since = None;
            }
            s.is_break = false;
        }

        if let Some(since) = s.running_since {
            s.elapsed = s.elapsed.saturating_add(now.saturating_sub(since));
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
pub fn toggle_break(app: tauri::AppHandle) -> Result<StopwatchState, AppError> {
    let now = current_epoch_millis()?;
    let updated = with_locked_state(&app, |s| {
        if s.is_break {
            if let Some(since) = s.break_running_since {
                s.break_elapsed = s.break_elapsed.saturating_add(now.saturating_sub(since));
                s.break_running_since = None;
            }
            s.is_break = false;
        } else {
            if let Some(since) = s.running_since {
                s.elapsed = s.elapsed.saturating_add(now.saturating_sub(since));
                s.running_since = None;
            }
            s.is_break = true;
            s.break_running_since = Some(now);
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
        s.is_break = false;
        s.break_elapsed = 0;
        s.break_running_since = None;
        s.paused_until = None;
        s.clone()
    })?;
    let _ = app.emit(crate::events::STOPWATCH_UPDATED, &updated);
    Ok(updated)
}
