#![warn(clippy::pedantic, clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use sqlx::SqlitePool;
use tauri::Manager;

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

pub(crate) fn db_pool(app: &tauri::AppHandle) -> Result<tauri::State<'_, SqlitePool>, AppError> {
    app.try_state::<SqlitePool>()
        .ok_or_else(|| AppError::NotReady("Database not initialized yet".to_string()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::too_many_lines, clippy::large_stack_frames)]
/// # Panics
///
/// Panics if the tauri application fails to run.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            dotenvy::dotenv().ok();
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
                        if let Some(main_win) = app.get_webview_window("main") {
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
                        if let Some(main_win) = app.get_webview_window("main") {
                            let _ = main_win.show();
                            let _ = main_win.unminimize();
                            let _ = main_win.set_focus();
                        }
                    }
                })
                .build(app)?;

            spawn_db_init(handle);
            Ok(())
        })
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::tasks::parse_sigils,
            commands::tasks::get_tasks,
            commands::tasks::create_task,
            commands::tasks::update_task,
            commands::tasks::delete_task,
            commands::events::get_events,
            commands::events::create_event,
            commands::events::update_event,
            commands::events::delete_event,
            commands::sync::force_sync,
            commands::sync::wipe_local_data,
            commands::sync::clear_sync_queue,
            commands::sync::get_sync_queue,
            screens::plan::get_plan_layout,
            screens::plan::get_filtered_tasks,
            screens::plan::get_filtered_events,
            screens::plan::get_task_schema,
            screens::plan::get_event_schema,
            commands::window::window_restore_main,
            commands::window::show_minitracker,
            commands::window::hide_launcher,
            commands::window::resize_launcher,
            auth::login_with_google,
            auth::is_logged_in,
            auth::reset_auth,
            stopwatch::get_stopwatch_state,
            stopwatch::toggle_stopwatch,
            stopwatch::reset_stopwatch,
            settings::get_settings_schema,
            settings::get_settings,
            settings::update_setting,
            sync::get_sync_state
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| panic!("error while running tauri application: {e}"));
}

fn spawn_db_init(handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let Ok(app_data_dir) = handle.path().app_data_dir() else {
            return;
        };
        if std::fs::create_dir_all(&app_data_dir).is_err() {
            eprintln!("Failed to create app data directory");
            return;
        }

        let db_path = app_data_dir.join("taskroot.db");
        let Some(db_url) = db_path.to_str().map(|p| format!("sqlite:{p}")) else {
            return;
        };

        match db::init_db(&db_url).await {
            Ok(pool) => {
                handle.manage(pool.clone());
                sync::start_sync_engine(handle, pool);
            }
            Err(e) => eprintln!("Failed to initialize DB: {e}"),
        }
    });
}
