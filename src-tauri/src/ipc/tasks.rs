use tauri_specta::Event;

use crate::error::AppError;
use crate::features::calendar as domain;
use crate::features::calendar::ops;
use crate::ipc::session::{client_for, store_state};
use crate::ipc_events::{StopwatchUpdated, StoreUpdated};

async fn clear_focus(app: &tauri::AppHandle, task_id: &str) {
    let Ok(Some(transition)) =
        crate::features::stopwatch::with_manager(app, |manager| manager.clear_task_if(task_id))
    else {
        return;
    };
    let _ = StopwatchUpdated(transition.state).emit(app);
    super::stopwatch::credit_time_spent(app, transition.entries).await;
}

#[tauri::command]
#[specta::specta]
#[must_use]
pub fn parse_sigils(task_name: String) -> domain::ParsedSigils {
    domain::parse_sigils(task_name)
}

/// # Errors
///
/// Returns an error when the account/store is unavailable or the `CalDAV` write
/// fails. The row is never kept locally on failure.
#[tauri::command]
#[specta::specta]
pub async fn create_task(app: tauri::AppHandle, task: domain::AppTask) -> Result<(), AppError> {
    let client = client_for(&app).await?;
    let state = store_state(&app)?;
    ops::create_task(&state, &client, task).await?;
    let _ = StoreUpdated.emit(&app);
    Ok(())
}

/// # Errors
///
/// Returns an error when the task is missing or the `CalDAV` write fails.
#[tauri::command]
#[specta::specta]
pub async fn update_task(
    app: tauri::AppHandle,
    id: String,
    patches: Vec<domain::TaskPatch>,
) -> Result<(), AppError> {
    let completed = domain::patches_mark_task_done(&patches);
    let client = client_for(&app).await?;
    let state = store_state(&app)?;
    ops::update_task(&state, &client, &id, &patches).await?;
    if completed {
        clear_focus(&app, &id).await;
    }
    let _ = StoreUpdated.emit(&app);
    Ok(())
}

/// # Errors
///
/// Returns an error when the task is missing or the `CalDAV` write fails.
#[tauri::command]
#[specta::specta]
pub async fn toggle_task(app: tauri::AppHandle, id: String) -> Result<(), AppError> {
    let client = client_for(&app).await?;
    let state = store_state(&app)?;
    let current_status = state
        .read(|store| store.task(&id).map(|task| task.status.clone()))
        .await
        .ok_or_else(|| AppError::NotFound(format!("task {id}")))?;

    let new_status = if current_status == Some(domain::AppTaskStatus::Done) {
        domain::AppTaskStatus::Todo
    } else {
        domain::AppTaskStatus::Done
    };
    let completed = new_status == domain::AppTaskStatus::Done;
    let patches = [domain::TaskPatch::Status(Some(new_status))];

    ops::update_task(&state, &client, &id, &patches).await?;
    if completed {
        clear_focus(&app, &id).await;
    }
    let _ = StoreUpdated.emit(&app);
    Ok(())
}

/// # Errors
///
/// Returns an error when the `CalDAV` delete fails.
#[tauri::command]
#[specta::specta]
pub async fn delete_task(app: tauri::AppHandle, id: String) -> Result<(), AppError> {
    let client = client_for(&app).await?;
    let state = store_state(&app)?;
    ops::delete_task(&state, &client, &id).await?;
    clear_focus(&app, &id).await;
    let _ = StoreUpdated.emit(&app);
    Ok(())
}

/// # Errors
///
/// Returns an error if the store is not initialized.
#[tauri::command]
#[specta::specta]
pub async fn get_past_due_task_ids(app: tauri::AppHandle) -> Result<Vec<String>, AppError> {
    let state = store_state(&app)?;
    Ok(state.read(ops::past_due_task_ids).await)
}
