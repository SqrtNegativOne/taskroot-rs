//! System tray icon and menu.

use std::error::Error;

use tauri::Manager;

use super::windows::{WindowLabel, restore_main};

/// Build the tray icon and its menu.
///
/// # Errors
///
/// Propagates Tauri's menu, icon or tray construction failure.
pub fn build_tray(app: &mut tauri::App) -> Result<(), Box<dyn Error>> {
    let open_i = tauri::menu::MenuItem::with_id(app, "open", "Open Taskroot", true, None::<&str>)?;
    let exit_i = tauri::menu::MenuItem::with_id(app, "exit", "Exit", true, None::<&str>)?;
    let menu = tauri::menu::Menu::with_items(app, &[&open_i, &exit_i])?;

    let mut tray_builder = tauri::tray::TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false);

    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    let _tray = tray_builder
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => restore_main(app),
            "exit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(main_win) = app.get_webview_window(WindowLabel::Main.as_str()) {
                    let _ = main_win.show();
                    let _ = main_win.unminimize();
                    let _ = main_win.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
