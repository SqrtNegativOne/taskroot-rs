use crate::commands::session::{client_for, store_state};
use crate::domain;
use crate::error::AppError;
use crate::store::ops;

#[tauri::command]
#[allow(clippy::needless_pass_by_value, clippy::must_use_candidate)]
pub fn parse_sigils(task_name: String) -> domain::ParsedSigils {
    domain::parse_sigils(&task_name)
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
    let client = client_for(&app).await?;
    let state = store_state(&app)?;
    let mut store = state.lock().await;
    ops::update_task(&mut store, &client, task).await?;
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
