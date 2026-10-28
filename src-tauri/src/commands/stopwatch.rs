use crate::error::AppError;
use crate::stopwatch::{current_epoch_millis, with_locked_state, StopwatchState};
use tauri::Emitter;

/// # Errors
///
/// Returns [`AppError::Internal`] when the stopwatch state is not managed yet or
/// the system clock cannot be read.
#[tauri::command]
pub fn get_stopwatch_state(app: tauri::AppHandle) -> Result<StopwatchState, AppError> {
    let now = current_epoch_millis()?;
    with_locked_state(&app, |state| {
        if state.paused_until.is_some_and(|until| until <= now) {
            state.paused_until = None;
        }
        state.clone()
    })
}

/// Toggle a timed pause. An active or expired pause is cleared; otherwise the
/// timer is paused for `pause_minutes`.
///
/// # Errors
///
/// Returns [`AppError::Internal`] when the stopwatch state is unavailable.
#[tauri::command]
pub fn toggle_pause(app: tauri::AppHandle, pause_minutes: u64) -> Result<StopwatchState, AppError> {
    let now = current_epoch_millis()?;
    let updated = with_locked_state(&app, |state| {
        if state.paused_until.is_some_and(|until| until > now) {
            state.paused_until = None;
        } else {
            let duration = pause_minutes.max(1).saturating_mul(60_000);
            state.paused_until = Some(now.saturating_add(duration));
        }
        state.clone()
    })?;
    let _ = app.emit(crate::events::STOPWATCH_UPDATED, &updated);
    Ok(updated)
}

/// Extend (`delta_minutes > 0`) or shrink an active pause by whole minutes. A
/// pause reduced to zero or below unpauses.
///
/// # Errors
///
/// Returns [`AppError::Internal`] when the stopwatch state is unavailable.
#[tauri::command]
pub fn adjust_pause(app: tauri::AppHandle, delta_minutes: i64) -> Result<StopwatchState, AppError> {
    let now = current_epoch_millis()?;
    let updated = with_locked_state(&app, |state| {
        if let Some(until) = state.paused_until {
            let delta = i128::from(delta_minutes).saturating_mul(60_000);
            let target = i128::from(until).saturating_add(delta);
            state.paused_until = if target <= i128::from(now) {
                None
            } else {
                u64::try_from(target).ok()
            };
        }
        state.clone()
    })?;
    let _ = app.emit(crate::events::STOPWATCH_UPDATED, &updated);
    Ok(updated)
}

/// # Errors
///
/// Returns [`AppError::Internal`] when the stopwatch state is unavailable.
#[tauri::command]
pub fn toggle_stopwatch(app: tauri::AppHandle) -> Result<StopwatchState, AppError> {
    let now = current_epoch_millis()?;
    let updated = with_locked_state(&app, |state| {
        if state.is_break {
            if let Some(since) = state.break_running_since {
                state.break_elapsed = state
                    .break_elapsed
                    .saturating_add(now.saturating_sub(since));
                state.break_running_since = None;
            }
            state.is_break = false;
        }

        if let Some(since) = state.running_since {
            state.elapsed = state.elapsed.saturating_add(now.saturating_sub(since));
            state.running_since = None;
        } else {
            state.running_since = Some(now);
        }

        state.clone()
    })?;
    let _ = app.emit(crate::events::STOPWATCH_UPDATED, &updated);
    Ok(updated)
}

/// # Errors
///
/// Returns [`AppError::Internal`] when the stopwatch state is unavailable.
#[tauri::command]
pub fn toggle_break(app: tauri::AppHandle) -> Result<StopwatchState, AppError> {
    let now = current_epoch_millis()?;
    let updated = with_locked_state(&app, |state| {
        if state.is_break {
            if let Some(since) = state.break_running_since {
                state.break_elapsed = state
                    .break_elapsed
                    .saturating_add(now.saturating_sub(since));
                state.break_running_since = None;
            }
            state.is_break = false;
        } else {
            if let Some(since) = state.running_since {
                state.elapsed = state.elapsed.saturating_add(now.saturating_sub(since));
                state.running_since = None;
            }
            state.is_break = true;
            state.break_running_since = Some(now);
        }

        state.clone()
    })?;
    let _ = app.emit(crate::events::STOPWATCH_UPDATED, &updated);
    Ok(updated)
}

/// # Errors
///
/// Returns [`AppError::Internal`] when the stopwatch state is unavailable.
#[tauri::command]
pub fn reset_stopwatch(app: tauri::AppHandle) -> Result<StopwatchState, AppError> {
    let updated = with_locked_state(&app, |state| {
        state.elapsed = 0;
        state.running_since = None;
        state.is_break = false;
        state.break_elapsed = 0;
        state.break_running_since = None;
        state.paused_until = None;
        state.clone()
    })?;
    let _ = app.emit(crate::events::STOPWATCH_UPDATED, &updated);
    Ok(updated)
}
