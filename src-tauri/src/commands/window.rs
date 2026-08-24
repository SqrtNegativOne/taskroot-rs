use tauri::Manager;

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn window_restore_main(app: tauri::AppHandle) {
    if let Some(main_win) = app.get_webview_window("main") {
        let _ = main_win.show();
        let _ = main_win.unminimize();
        let _ = main_win.set_focus();
    }
    if let Some(mini_win) = app.get_webview_window("minitracker") {
        let _ = mini_win.hide();
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn show_minitracker(app: tauri::AppHandle) {
    if let Some(mini_win) = app.get_webview_window("minitracker") {
        let _ = mini_win.show();
        let _ = mini_win.unminimize();
        let _ = mini_win.set_focus();
    }
    if let Some(main_win) = app.get_webview_window("main") {
        let _ = main_win.hide();
    }
}
