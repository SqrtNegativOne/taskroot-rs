use tauri_specta::Event;

use crate::error::AppError;
use crate::features::stopwatch::{
    with_manager, StopwatchLabelSource, StopwatchManager, StopwatchState,
};
use crate::ipc_events::StopwatchUpdated;

pub(crate) fn clear_account_focus(app: &tauri::AppHandle) {
    let result = with_manager(app, |manager| {
        let snapshot = manager.snapshot()?;
        snapshot
            .task_id
            .map_or(Ok(None), |id| manager.clear_task_if(&id))
    });
    match result {
        Ok(Some(updated)) => emit(app, &updated),
        Ok(None) => {}
        Err(error) => eprintln!("Could not clear the previous account's stopwatch focus: {error}"),
    }
}

fn emit(app: &tauri::AppHandle, state: &StopwatchState) {
    let _ = StopwatchUpdated(state.clone()).emit(app);
}

/// # Errors
///
/// Returns [`AppError::Internal`] when the stopwatch state is not managed yet or
/// the system clock cannot be read.
#[tauri::command]
#[specta::specta]
pub fn get_stopwatch_state(app: tauri::AppHandle) -> Result<StopwatchState, AppError> {
    with_manager(&app, StopwatchManager::snapshot)
}

/// Start, pause or resume work.
///
/// # Errors
///
/// Returns [`AppError::InvalidInput`] while a break is active.
#[tauri::command]
#[specta::specta]
pub fn toggle_stopwatch(app: tauri::AppHandle) -> Result<StopwatchState, AppError> {
    let updated = with_manager(&app, StopwatchManager::toggle_work)?;
    emit(&app, &updated);
    Ok(updated)
}

/// Start a break from the work done so far, or cancel an active break.
///
/// # Errors
///
/// Returns [`AppError::InvalidInput`] when the session is too short or its break
/// already finished.
#[tauri::command]
#[specta::specta]
pub fn toggle_break(app: tauri::AppHandle, break_divisor: u32) -> Result<StopwatchState, AppError> {
    let updated = with_manager(&app, |manager| manager.toggle_break(break_divisor))?;
    emit(&app, &updated);
    Ok(updated)
}

/// Move a finished break countdown to the terminal `Stopped` state.
///
/// # Errors
///
/// Returns [`AppError::Internal`] when the stopwatch state is unavailable.
#[tauri::command]
#[specta::specta]
pub fn complete_break(app: tauri::AppHandle) -> Result<StopwatchState, AppError> {
    let updated = with_manager(&app, StopwatchManager::complete_break)?;
    emit(&app, &updated);
    Ok(updated)
}

/// Toggle a timed pause. An active or expired pause is cleared; otherwise the
/// timer is paused for `pause_minutes`.
///
/// # Errors
///
/// Returns [`AppError::Internal`] when the stopwatch state is unavailable.
#[tauri::command]
#[specta::specta]
pub fn toggle_pause(app: tauri::AppHandle, pause_minutes: u64) -> Result<StopwatchState, AppError> {
    let updated = with_manager(&app, |manager| manager.toggle_pause(pause_minutes))?;
    emit(&app, &updated);
    Ok(updated)
}

/// Extend (`delta_minutes > 0`) or shrink an active pause by whole minutes. A
/// pause reduced to zero or below unpauses.
///
/// # Errors
///
/// Returns [`AppError::Internal`] when the stopwatch state is unavailable.
#[tauri::command]
#[specta::specta]
pub fn adjust_pause(app: tauri::AppHandle, delta_minutes: i64) -> Result<StopwatchState, AppError> {
    let updated = with_manager(&app, |manager| manager.adjust_pause(delta_minutes))?;
    emit(&app, &updated);
    Ok(updated)
}

/// Set or clear the mini tracker label recorded with work and break history.
/// Setting a label clears the local focus task.
///
/// # Errors
///
/// Returns [`AppError::Internal`] when the stopwatch state is unavailable.
#[tauri::command]
#[specta::specta]
pub fn set_stopwatch_label(
    app: tauri::AppHandle,
    label: Option<String>,
    source: StopwatchLabelSource,
) -> Result<StopwatchState, AppError> {
    let updated = with_manager(&app, |manager| manager.set_label(label, source))?;
    emit(&app, &updated);
    Ok(updated)
}

/// Point the stopwatch at a task, or clear the local focus pointer. This never
/// writes the task's `CalDAV` status.
///
/// # Errors
///
/// Returns [`AppError::Internal`] when the stopwatch state is unavailable.
#[tauri::command]
#[specta::specta]
pub fn set_stopwatch_task(
    app: tauri::AppHandle,
    task_id: Option<String>,
    label: Option<String>,
) -> Result<StopwatchState, AppError> {
    let updated = with_manager(&app, |manager| manager.set_task(task_id, label))?;
    emit(&app, &updated);
    Ok(updated)
}

/// # Errors
///
/// Returns [`AppError::Internal`] when the stopwatch state is unavailable.
#[tauri::command]
#[specta::specta]
pub fn reset_stopwatch(app: tauri::AppHandle) -> Result<StopwatchState, AppError> {
    let updated = with_manager(&app, StopwatchManager::reset)?;
    emit(&app, &updated);
    Ok(updated)
}
