//! Collection commands; account lifecycle is in `ipc::accounts`.

use crate::error::AppError;
use crate::features::calendar::{calendars, CalendarComponent};
use tauri::AppHandle;

/// # Errors
/// Returns account, MKCALENDAR or discovery failures.
#[tauri::command]
pub async fn create_calendar(
    app: AppHandle,
    display_name: String,
    component: CalendarComponent,
) -> Result<(), AppError> {
    let dir = crate::platform::app_data_dir(&app)?;
    let client = super::session::client_for(&app).await?;
    let state = super::session::store_state(&app)?;
    let mut store = state.lock().await;
    calendars::create(&dir, &client, &mut store, &display_name, component).await
}

/// # Errors
/// Returns unknown/read-only collection or PROPPATCH failures.
#[tauri::command]
pub async fn rename_calendar(
    app: AppHandle,
    href: String,
    display_name: String,
) -> Result<(), AppError> {
    let dir = crate::platform::app_data_dir(&app)?;
    let client = super::session::client_for(&app).await?;
    let state = super::session::store_state(&app)?;
    let mut store = state.lock().await;
    calendars::rename(&dir, &client, &mut store, &href, &display_name).await
}

/// # Errors
/// Returns invalid-color, read-only collection or PROPPATCH failures.
#[tauri::command]
pub async fn set_calendar_color(
    app: AppHandle,
    href: String,
    color: Option<String>,
) -> Result<(), AppError> {
    let dir = crate::platform::app_data_dir(&app)?;
    let client = super::session::client_for(&app).await?;
    let state = super::session::store_state(&app)?;
    let mut store = state.lock().await;
    calendars::set_color(&dir, &client, &mut store, &href, color).await
}

/// # Errors
/// Returns unknown/read-only collection or DELETE failures.
#[tauri::command]
pub async fn delete_calendar(app: AppHandle, href: String) -> Result<(), AppError> {
    let dir = crate::platform::app_data_dir(&app)?;
    let client = super::session::client_for(&app).await?;
    let state = super::session::store_state(&app)?;
    let mut store = state.lock().await;
    calendars::delete(&dir, &client, &mut store, &href).await
}
