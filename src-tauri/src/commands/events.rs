use crate::commands::session::{app_calendar, client_for, store_state};
use crate::domain;
use crate::error::AppError;
use crate::store::ops;

/// # Errors
///
/// Returns an error when the account/store is unavailable or the `CalDAV` write
/// fails. The row is never kept locally on failure.
#[tauri::command]
pub async fn create_event(
    app: tauri::AppHandle,
    event: domain::AppEvent,
) -> Result<(), AppError> {
    let client = client_for(&app).await?;
    let state = store_state(&app)?;
    let mut store = state.lock().await;
    ops::create_event(&mut store, &client, event).await?;
    Ok(())
}

/// # Errors
///
/// Returns an error when the event is missing or the `CalDAV` write fails.
#[tauri::command]
pub async fn update_event(
    app: tauri::AppHandle,
    event: domain::AppEvent,
) -> Result<(), AppError> {
    let client = client_for(&app).await?;
    let state = store_state(&app)?;
    let mut store = state.lock().await;
    ops::update_event(&mut store, &client, event).await?;
    Ok(())
}

/// # Errors
///
/// Returns an error when the `CalDAV` delete fails.
#[tauri::command]
pub async fn delete_event(app: tauri::AppHandle, id: String) -> Result<(), AppError> {
    let client = client_for(&app).await?;
    let state = store_state(&app)?;
    let mut store = state.lock().await;
    ops::delete_event(&mut store, &client, &id).await?;
    Ok(())
}

/// The store's discovered calendars, projected for the frontend.
///
/// # Errors
///
/// Returns an error if the store is not initialized.
#[tauri::command]
pub async fn get_active_calendars(
    app: tauri::AppHandle,
) -> Result<Vec<domain::AppCalendar>, AppError> {
    let state = store_state(&app)?;
    Ok(state
        .read(|store| store.calendars.iter().map(app_calendar).collect())
        .await)
}
