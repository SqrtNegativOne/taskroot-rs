//! Window control commands: thin wrappers over `platform::windows`.

use crate::error::AppError;
use crate::platform;

#[tauri::command]
pub fn window_restore_main(app: tauri::AppHandle) {
    platform::restore_main(&app);
}

/// # Errors
///
/// Returns an internal error if the mini tracker cannot be created or shown.
#[tauri::command]
pub async fn set_minitracker_visible(
    app: tauri::AppHandle,
    visible: bool,
) -> Result<(), AppError> {
    platform::set_minitracker_visible(&app, visible)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))
}

#[tauri::command]
pub async fn hide_launcher(app: tauri::AppHandle) {
    platform::hide_launcher(&app).await;
}

/// # Errors
///
/// Returns an internal error if the screen-effect overlay cannot be created or shown.
#[tauri::command]
pub async fn show_screen_effect(app: tauri::AppHandle) -> Result<(), AppError> {
    platform::show_screen_effect(&app)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))
}

#[tauri::command]
pub async fn hide_screen_effect(app: tauri::AppHandle) {
    platform::hide_screen_effect(&app).await;
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    platform::quit_app(&app);
}

/// # Errors
///
/// Returns an internal error if the launcher cannot be created or shown.
#[tauri::command]
pub async fn toggle_launcher(app: tauri::AppHandle) -> Result<(), AppError> {
    platform::toggle_launcher_window(&app)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))
}

#[tauri::command]
pub fn resize_launcher(app: tauri::AppHandle, width: f64, height: f64) {
    platform::resize_launcher(&app, width, height);
}
