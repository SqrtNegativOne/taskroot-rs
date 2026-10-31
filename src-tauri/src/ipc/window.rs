//! Window control commands: thin wrappers over `platform::windows`.

use crate::platform;

#[tauri::command]
pub fn window_restore_main(app: tauri::AppHandle) {
    platform::restore_main(&app);
}

#[tauri::command]
pub fn show_minitracker(app: tauri::AppHandle) {
    platform::show_minitracker(&app);
}

#[tauri::command]
pub fn hide_launcher(app: tauri::AppHandle) {
    platform::hide_launcher(&app);
}

#[tauri::command]
pub fn show_screen_effect(app: tauri::AppHandle) {
    platform::show_screen_effect(&app);
}

#[tauri::command]
pub fn hide_screen_effect(app: tauri::AppHandle) {
    platform::hide_screen_effect(&app);
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    platform::quit_app(&app);
}

#[tauri::command]
pub fn toggle_launcher(app: tauri::AppHandle) {
    platform::toggle_launcher_window(&app);
}

#[tauri::command]
pub fn resize_launcher(app: tauri::AppHandle, width: f64, height: f64) {
    platform::resize_launcher(&app, width, height);
}
