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

    let icon = tauri::image::Image::from_bytes(include_bytes!("../../icons/32x32.png"))?;

    let tray = tauri::tray::TrayIconBuilder::with_id("taskroot")
        .icon(icon)
        .tooltip("Taskroot")
        .menu(&menu)
        .show_menu_on_left_click(false)
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

    tray.set_visible(true)?;
    if cfg!(debug_assertions) {
        let handle = app.handle().clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            if let Some(tray) = handle.tray_by_id("taskroot") {
                let modified = tray.set_tooltip(Some("Taskroot"));
                eprintln!("[tray] modify={modified:?} rect={:?}", tray.rect());
            } else {
                eprintln!("[tray] tray_by_id returned None");
            }
        });
    }

    Ok(())
}
