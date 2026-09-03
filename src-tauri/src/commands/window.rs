use crate::domain::WindowLabel;
use tauri::Manager;

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn window_restore_main(app: tauri::AppHandle) {
    if let Some(main_win) = app.get_webview_window(WindowLabel::Main.as_str()) {
        let _ = main_win.show();
        let _ = main_win.unminimize();
        let _ = main_win.set_focus();
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn show_minitracker(app: tauri::AppHandle) {
    if let Some(mini_win) = app.get_webview_window(WindowLabel::Minitracker.as_str()) {
        let _ = mini_win.show();
        let _ = mini_win.unminimize();
        let _ = mini_win.set_focus();
        let _ = mini_win.set_always_on_top(true);
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn hide_launcher(app: tauri::AppHandle) {
    if let Some(launcher_win) = app.get_webview_window(WindowLabel::Launcher.as_str()) {
        let _ = launcher_win.hide();
    }
}

pub fn toggle_launcher_window(app: &tauri::AppHandle) {
    let Some(launcher_win) = app.get_webview_window(WindowLabel::Launcher.as_str()) else {
        return;
    };

    if matches!(launcher_win.is_visible(), Ok(true)) {
        let _ = launcher_win.hide();
        return;
    }

    let _ = launcher_win.center();
    let _ = launcher_win.show();
    let _ = launcher_win.set_focus();
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn toggle_launcher(app: tauri::AppHandle) {
    toggle_launcher_window(&app);
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn resize_launcher(app: tauri::AppHandle, width: f64, height: f64) {
    if let Some(launcher_win) = app.get_webview_window(WindowLabel::Launcher.as_str()) {
        let _ = launcher_win.set_size(tauri::Size::Logical(tauri::LogicalSize::new(width, height)));
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use tauri_plugin_global_shortcut::Shortcut;

    #[test]
    fn test_shortcut_parsing() {
        let Ok(s1) = Shortcut::from_str("CommandOrControl+Shift+Space") else {
            panic!("failed to parse CommandOrControl+Shift+Space");
        };
        let Ok(s2) = Shortcut::from_str("Super+Shift+Space") else {
            panic!("failed to parse Super+Shift+Space");
        };
        assert_ne!(s1, s2);
        let s3 = Shortcut::from_str("Meta+Shift+Space");
        assert!(s3.is_err());
    }
}
