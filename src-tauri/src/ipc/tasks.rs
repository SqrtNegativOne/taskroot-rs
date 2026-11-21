use tauri::Emitter;

use crate::error::AppError;
use crate::features::calendar as domain;
use crate::features::calendar::ops;
use crate::ipc::session::{client_for, store_state};

fn clear_focus(app: &tauri::AppHandle, task_id: &str) {
    let Ok(Some(updated)) =
        crate::features::stopwatch::with_manager(app, |manager| manager.clear_task_if(task_id))
    else {
        return;
    };
    let _ = app.emit(crate::ipc_events::STOPWATCH_UPDATED, updated);
}

#[tauri::command]
#[must_use]
pub fn parse_sigils(task_name: String) -> domain::ParsedSigils {
    domain::parse_sigils(task_name)
}

/// # Errors
///
/// Returns an error when the account/store is unavailable or the `CalDAV` write
/// fails. The row is never kept locally on failure.
#[tauri::command]
pub async fn create_task(app: tauri::AppHandle, task: domain::AppTask) -> Result<(), AppError> {
    let client = client_for(&app).await?;
    let state = store_state(&app)?;
    let mut store = state.lock().await;
    ops::create_task(&mut store, &client, task).await?;
    Ok(())
}

/// # Errors
///
/// Returns an error when the task is missing or the `CalDAV` write fails.
#[tauri::command]
pub async fn update_task(app: tauri::AppHandle, task: domain::AppTask) -> Result<(), AppError> {
    let task_id = task.id.0.clone();
    let completed = task.status == Some(domain::AppTaskStatus::Done);
    let client = client_for(&app).await?;
    let state = store_state(&app)?;
    let mut store = state.lock().await;
    ops::update_task(&mut store, &client, task).await?;
    drop(store);
    if completed {
        clear_focus(&app, &task_id);
    }
    Ok(())
}

/// # Errors
///
/// Returns an error when the task is missing or the `CalDAV` write fails.
#[tauri::command]
pub async fn toggle_task(app: tauri::AppHandle, id: String) -> Result<(), AppError> {
    let client = client_for(&app).await?;
    let state = store_state(&app)?;
    let mut store = state.lock().await;
    let mut task = store
        .task(&id)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("task {id}")))?;

    let new_status = if task.status == Some(domain::AppTaskStatus::Done) {
        domain::AppTaskStatus::Todo
    } else {
        domain::AppTaskStatus::Done
    };
    let completed = new_status == domain::AppTaskStatus::Done;
    task.status = Some(new_status);

    ops::update_task(&mut store, &client, task).await?;
    drop(store);
    if completed {
        clear_focus(&app, &id);
    }
    Ok(())
}

/// # Errors
///
/// Returns an error when the `CalDAV` delete fails.
#[tauri::command]
pub async fn delete_task(app: tauri::AppHandle, id: String) -> Result<(), AppError> {
    let client = client_for(&app).await?;
    let state = store_state(&app)?;
    let mut store = state.lock().await;
    ops::delete_task(&mut store, &client, &id).await?;
    drop(store);
    clear_focus(&app, &id);
    Ok(())
}

/// # Errors
///
/// Returns an error if the store is not initialized.
#[tauri::command]
pub async fn get_past_due_task_ids(app: tauri::AppHandle) -> Result<Vec<String>, AppError> {
    let state = store_state(&app)?;
    Ok(state.read(ops::past_due_task_ids).await)
}
