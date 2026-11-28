use std::collections::BTreeMap;
use std::time::Duration;

use tauri_specta::Event;

use crate::error::AppError;
use crate::features::calendar::{ops, CalDavClient, StoreState, TaskPatch};
use crate::features::stopwatch::history::{StopwatchHistoryEntry, StopwatchHistoryKind};
use crate::features::stopwatch::{
    with_manager, StopwatchLabelSource, StopwatchManager, StopwatchState,
};
use crate::ipc::session::{client_for, store_state};
use crate::ipc_events::{StopwatchUpdated, StoreUpdated};

const EXIT_CREDIT_TIMEOUT: Duration = Duration::from_secs(3);

pub(crate) fn aggregate_work_seconds(entries: &[StopwatchHistoryEntry]) -> BTreeMap<String, i32> {
    let mut totals_ms: BTreeMap<&str, u64> = BTreeMap::new();
    for entry in entries {
        if entry.kind != StopwatchHistoryKind::Work || entry.duration_ms == 0 {
            continue;
        }
        let Some(task_id) = entry.task_id.as_deref() else {
            continue;
        };
        let entry_total = totals_ms.entry(task_id).or_insert(0);
        *entry_total = entry_total.saturating_add(entry.duration_ms);
    }

    let mut seconds = BTreeMap::new();
    for (task_id, total_ms) in totals_ms {
        let rounded = total_ms.saturating_add(500).checked_div(1_000).unwrap_or(0);
        if rounded == 0 {
            continue;
        }
        seconds.insert(
            task_id.to_string(),
            i32::try_from(rounded).unwrap_or(i32::MAX),
        );
    }
    seconds
}

pub(crate) async fn apply_time_spent_credits(
    state: &StoreState,
    client: &CalDavClient,
    credits: &BTreeMap<String, i32>,
) -> bool {
    let mut updated_any = false;
    for (task_id, added_secs) in credits {
        let prev = state
            .read(|store| store.task(task_id).map(|task| task.time_spent))
            .await;
        let Some(prev_opt) = prev else {
            continue;
        };
        let prev_secs = prev_opt.unwrap_or(0).max(0);
        let next = prev_secs.saturating_add(*added_secs);
        if let Err(error) =
            ops::update_task(state, client, task_id, &[TaskPatch::TimeSpent(Some(next))]).await
        {
            eprintln!("Failed to credit stopwatch time for task {task_id}: {error}");
        } else {
            updated_any = true;
        }
    }
    updated_any
}

pub(crate) async fn credit_time_spent(
    app: &tauri::AppHandle,
    entries: Vec<StopwatchHistoryEntry>,
) {
    let credits = aggregate_work_seconds(&entries);
    if credits.is_empty() {
        return;
    }
    let state = match store_state(app) {
        Ok(state) => state,
        Err(error) => {
            eprintln!("Cannot credit stopwatch time: {error}");
            return;
        }
    };
    let client = match client_for(app).await {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Cannot credit stopwatch time: {error}");
            return;
        }
    };
    if apply_time_spent_credits(&state, &client, &credits).await {
        let _ = StoreUpdated.emit(app);
    }
}

pub(crate) async fn flush_and_credit_on_exit(app: &tauri::AppHandle) {
    let Ok(transition) = with_manager(app, StopwatchManager::flush_for_exit) else {
        return;
    };
    emit(app, &transition.state);
    if !transition.entries.is_empty() {
        let _ = tokio::time::timeout(
            EXIT_CREDIT_TIMEOUT,
            credit_time_spent(app, transition.entries),
        )
        .await;
    }
}

pub(crate) async fn clear_account_focus(app: &tauri::AppHandle) {
    let result = with_manager(app, |manager| {
        let snapshot = manager.snapshot()?;
        snapshot
            .task_id
            .map_or(Ok(None), |id| manager.clear_task_if(&id))
    });
    match result {
        Ok(Some(transition)) => {
            emit(app, &transition.state);
            credit_time_spent(app, transition.entries).await;
        }
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

/// Return all persisted stopwatch work and break history entries.
///
/// # Errors
///
/// Returns [`AppError::Internal`] when the stopwatch state is not managed yet or
/// [`AppError::Store`] when history cannot be read.
#[tauri::command]
#[specta::specta]
pub fn get_stopwatch_history(app: tauri::AppHandle) -> Result<Vec<StopwatchHistoryEntry>, AppError> {
    with_manager(&app, StopwatchManager::history)
}

/// Start, pause or resume work.
///
/// # Errors
///
/// Returns [`AppError::InvalidInput`] while a break is active.
#[tauri::command]
#[specta::specta]
pub async fn toggle_stopwatch(app: tauri::AppHandle) -> Result<StopwatchState, AppError> {
    let transition = with_manager(&app, StopwatchManager::toggle_work)?;
    emit(&app, &transition.state);
    credit_time_spent(&app, transition.entries).await;
    Ok(transition.state)
}

/// Start a break from the work done so far, or cancel an active break.
///
/// # Errors
///
/// Returns [`AppError::InvalidInput`] when the session is too short or its break
/// already finished.
#[tauri::command]
#[specta::specta]
pub async fn toggle_break(
    app: tauri::AppHandle,
    break_divisor: u32,
) -> Result<StopwatchState, AppError> {
    let transition = with_manager(&app, |manager| manager.toggle_break(break_divisor))?;
    emit(&app, &transition.state);
    credit_time_spent(&app, transition.entries).await;
    Ok(transition.state)
}

/// Move a finished break countdown to the terminal `Stopped` state.
///
/// # Errors
///
/// Returns [`AppError::Internal`] when the stopwatch state is unavailable.
#[tauri::command]
#[specta::specta]
pub fn complete_break(app: tauri::AppHandle) -> Result<StopwatchState, AppError> {
    let transition = with_manager(&app, StopwatchManager::complete_break)?;
    emit(&app, &transition.state);
    Ok(transition.state)
}

/// Toggle a timed pause. An active or expired pause is cleared; otherwise the
/// timer is paused for `pause_minutes`.
///
/// # Errors
///
/// Returns [`AppError::Internal`] when the stopwatch state is unavailable.
#[tauri::command]
#[specta::specta]
pub async fn toggle_pause(
    app: tauri::AppHandle,
    pause_minutes: u64,
) -> Result<StopwatchState, AppError> {
    let transition = with_manager(&app, |manager| manager.toggle_pause(pause_minutes))?;
    emit(&app, &transition.state);
    credit_time_spent(&app, transition.entries).await;
    Ok(transition.state)
}

/// Extend (`delta_minutes > 0`) or shrink an active timed pause by whole minutes. A
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
pub async fn set_stopwatch_label(
    app: tauri::AppHandle,
    label: Option<String>,
    source: StopwatchLabelSource,
) -> Result<StopwatchState, AppError> {
    let transition = with_manager(&app, |manager| manager.set_label(label, source))?;
    emit(&app, &transition.state);
    credit_time_spent(&app, transition.entries).await;
    Ok(transition.state)
}

/// Point the stopwatch at a task, or clear the local focus pointer. This never
/// writes the task's `CalDAV` status.
///
/// # Errors
///
/// Returns [`AppError::Internal`] when the stopwatch state is unavailable.
#[tauri::command]
#[specta::specta]
pub async fn set_stopwatch_task(
    app: tauri::AppHandle,
    task_id: Option<String>,
    label: Option<String>,
) -> Result<StopwatchState, AppError> {
    let transition = with_manager(&app, |manager| manager.set_task(task_id, label))?;
    emit(&app, &transition.state);
    credit_time_spent(&app, transition.entries).await;
    Ok(transition.state)
}

/// # Errors
///
/// Returns [`AppError::Internal`] when the stopwatch state is unavailable.
#[tauri::command]
#[specta::specta]
pub async fn reset_stopwatch(app: tauri::AppHandle) -> Result<StopwatchState, AppError> {
    let transition = with_manager(&app, StopwatchManager::reset)?;
    emit(&app, &transition.state);
    credit_time_spent(&app, transition.entries).await;
    Ok(transition.state)
}

#[cfg(test)]
mod tests;
