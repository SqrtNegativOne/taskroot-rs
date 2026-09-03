#![warn(clippy::pedantic, clippy::nursery)]
#![warn(unreachable_pub)]
#![allow(clippy::missing_errors_doc)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![allow(clippy::exit, clippy::panic)]

use sqlx::SqlitePool;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

pub mod apis;
pub mod auth;
pub mod commands;
pub mod db;
pub mod domain;
pub mod error;
pub mod events;
pub mod screens;
pub mod settings;
pub mod stopwatch;
pub mod sync;
pub mod time_utils;

use error::AppError;

pub const LAUNCHER_SHORTCUT: &str = "CommandOrControl+Shift+Space";

pub fn toggle_launcher(app: &tauri::AppHandle) {
    commands::window::toggle_launcher_window(app);
}

#[must_use]
pub fn extract_deep_link_route(arg: &str) -> Option<&str> {
    let route = arg.strip_prefix("taskroot://")?;
    Some(route.trim_end_matches('/'))
}

pub(crate) fn db_pool(app: &tauri::AppHandle) -> Result<tauri::State<'_, SqlitePool>, AppError> {
    app.try_state::<SqlitePool>()
        .ok_or_else(|| AppError::Internal("Database not initialized yet".to_string()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::too_many_lines, clippy::large_stack_frames)]
/// # Panics
///
/// Panics if the tauri application fails to run.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_deep_link::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        toggle_launcher(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(window) = app.get_webview_window(domain::WindowLabel::Main.as_str()) {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
            for arg in &args {
                if let Some(route) = extract_deep_link_route(arg) {
                    let _ = app.emit(events::DEEP_LINK, route);
                }
            }
        }))
        .setup(|app| {
            dotenvy::dotenv().ok();
            let _ = app.global_shortcut().register(LAUNCHER_SHORTCUT);
            app.manage(auth::AuthState::default());
            app.manage(stopwatch::StopwatchManager(std::sync::Mutex::new(
                stopwatch::StopwatchState::default(),
            )));
            app.manage(sync::SyncStateManager(std::sync::Mutex::new(
                sync::SyncState::default(),
            )));
            let handle = app.handle().clone();

            let open_i =
                tauri::menu::MenuItem::with_id(app, "open", "Open Taskroot", true, None::<&str>)?;
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
                    "open" => {
                        if let Some(main_win) =
                            app.get_webview_window(domain::WindowLabel::Main.as_str())
                        {
                            let _ = main_win.show();
                            let _ = main_win.unminimize();
                            let _ = main_win.set_focus();
                        }
                    }
                    "exit" => {
                        app.exit(0);
                    }
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
                        if let Some(main_win) =
                            app.get_webview_window(domain::WindowLabel::Main.as_str())
                        {
                            let _ = main_win.show();
                            let _ = main_win.unminimize();
                            let _ = main_win.set_focus();
                        }
                    }
                })
                .build(app)?;

            let app_data_dir = handle.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;

            let db_path = app_data_dir.join("taskroot.db");
            let db_url = format!("sqlite:{}", db_path.to_str().ok_or("Invalid db path")?);

            let pool = tauri::async_runtime::block_on(async { db::init_db(&db_url).await })?;
            handle.manage(pool.clone());
            sync::start_sync_engine(handle, pool);

            Ok(())
        })
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == domain::WindowLabel::Main.as_str() {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::tasks::parse_sigils,
            commands::tasks::create_task,
            commands::tasks::update_task,
            commands::tasks::delete_task,
            commands::tasks::get_past_due_task_ids,
            commands::events::create_event,
            commands::events::update_event,
            commands::events::delete_event,
            commands::events::get_active_calendars,
            commands::sync::force_sync,
            commands::sync::wipe_local_data,
            commands::sync::clear_sync_queue,
            commands::sync::get_sync_queue,
            screens::plan::query_tasks,
            screens::plan::query_events,
            screens::plan::get_task_schema,
            screens::plan::get_event_schema,
            commands::window::window_restore_main,
            commands::window::show_minitracker,
            commands::window::hide_launcher,
            commands::window::toggle_launcher,
            commands::window::resize_launcher,
            auth::login_with_google,
            auth::is_logged_in,
            auth::reset_auth,
            stopwatch::get_stopwatch_state,
            stopwatch::toggle_stopwatch,
            stopwatch::toggle_break,
            stopwatch::reset_stopwatch,
            settings::get_settings_schema,
            settings::get_settings,
            settings::update_setting,
            sync::get_sync_state
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| panic!("error while running tauri application: {e}"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_deep_link_route() {
        assert_eq!(extract_deep_link_route("taskroot://day"), Some("day"));
        assert_eq!(extract_deep_link_route("taskroot://day/"), Some("day"));
        assert_eq!(
            extract_deep_link_route("taskroot://settings"),
            Some("settings")
        );
        assert_eq!(extract_deep_link_route("taskroot://"), Some(""));
        assert_eq!(extract_deep_link_route("taskroot:///"), Some(""));
        assert_eq!(extract_deep_link_route("https://example.com"), None);
        assert_eq!(extract_deep_link_route("--other-arg"), None);
    }
}
