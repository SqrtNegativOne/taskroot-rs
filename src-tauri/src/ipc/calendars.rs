//! Collection commands; account lifecycle is in `ipc::accounts`.

use tauri::AppHandle;
use tauri_specta::Event;

use crate::error::AppError;
use crate::features::calendar::{calendars, CalendarComponent};
use crate::ipc_events::StoreUpdated;

/// # Errors
/// Returns account, MKCALENDAR or discovery failures.
#[tauri::command]
#[specta::specta]
pub async fn create_calendar(
    app: AppHandle,
    display_name: String,
    component: CalendarComponent,
) -> Result<(), AppError> {
    let dir = crate::platform::app_data_dir(&app)?;
    let client = super::session::client_for(&app).await?;
    let state = super::session::store_state(&app)?;
    calendars::create(&dir, &client, &state, &display_name, component).await?;
    let _ = StoreUpdated.emit(&app);
    Ok(())
}

/// # Errors
/// Returns unknown/read-only collection or PROPPATCH failures.
#[tauri::command]
#[specta::specta]
pub async fn rename_calendar(
    app: AppHandle,
    href: String,
    display_name: String,
) -> Result<(), AppError> {
    let dir = crate::platform::app_data_dir(&app)?;
    let client = super::session::client_for(&app).await?;
    let state = super::session::store_state(&app)?;
    calendars::rename(&dir, &client, &state, &href, &display_name).await?;
    let _ = StoreUpdated.emit(&app);
    Ok(())
}

/// # Errors
/// Returns invalid-color, read-only collection or PROPPATCH failures.
#[tauri::command]
#[specta::specta]
pub async fn set_calendar_color(
    app: AppHandle,
    href: String,
    color: Option<String>,
) -> Result<(), AppError> {
    let dir = crate::platform::app_data_dir(&app)?;
    let client = super::session::client_for(&app).await?;
    let state = super::session::store_state(&app)?;
    calendars::set_color(&dir, &client, &state, &href, color).await?;
    let _ = StoreUpdated.emit(&app);
    Ok(())
}

/// # Errors
/// Returns unknown/read-only collection or DELETE failures.
#[tauri::command]
#[specta::specta]
pub async fn delete_calendar(app: AppHandle, href: String) -> Result<(), AppError> {
    let dir = crate::platform::app_data_dir(&app)?;
    let client = super::session::client_for(&app).await?;
    let state = super::session::store_state(&app)?;
    calendars::delete(&dir, &client, &state, &href).await?;
    let _ = StoreUpdated.emit(&app);
    Ok(())
}
