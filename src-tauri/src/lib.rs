#![warn(clippy::pedantic, clippy::nursery)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use tauri::{Manager, Emitter};

pub mod db;
pub mod apis;
pub mod domain;
pub mod screens;
pub mod auth;
pub mod sync;
pub mod time_utils;
pub mod stopwatch;
pub mod settings;

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn parse_sigils(task_name: String) -> domain::ParsedSigils {
    domain::parse_sigils(&task_name)
}

#[tauri::command]
async fn get_tasks(app: tauri::AppHandle) -> Result<Vec<domain::AppTask>, String> {
    let pool = app
        .try_state::<sqlx::SqlitePool>()
        .ok_or("Database not initialized yet")?;
    db::get_tasks(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_events(app: tauri::AppHandle) -> Result<Vec<domain::AppEvent>, String> {
    let pool = app
        .try_state::<sqlx::SqlitePool>()
        .ok_or("Database not initialized yet")?;
    db::get_events(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_task(app: tauri::AppHandle, mut task: domain::AppTask) -> Result<(), String> {
    let pool = app
        .try_state::<sqlx::SqlitePool>()
        .ok_or("Database not initialized yet")?;
        
    task.dirty = Some(true); // default to dirty

    if let Ok(token) = auth::get_valid_access_token(&pool).await {
        if let Ok(remote_id) = apis::google_tasks::publish(&task, &token).await {
            task.remote_id = Some(remote_id);
            task.dirty = Some(false);
        }
    }

    db::create_task(&pool, task)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_task(app: tauri::AppHandle, mut task: domain::AppTask) -> Result<(), String> {
    let pool = app
        .try_state::<sqlx::SqlitePool>()
        .ok_or("Database not initialized yet")?;
        
    task.dirty = Some(true);

    if let Ok(token) = auth::get_valid_access_token(&pool).await {
        if let Ok(remote_id) = apis::google_tasks::publish(&task, &token).await {
            task.remote_id = Some(remote_id);
            task.dirty = Some(false);
        }
    }

    db::update_task(&pool, task)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_task(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let pool = app
        .try_state::<sqlx::SqlitePool>()
        .ok_or("Database not initialized yet")?;
    db::delete_task(&pool, id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_event(app: tauri::AppHandle, mut event: domain::AppEvent) -> Result<(), String> {
    let pool = app
        .try_state::<sqlx::SqlitePool>()
        .ok_or("Database not initialized yet")?;
        
    event.dirty = Some(true);

    if let Ok(token) = auth::get_valid_access_token(&pool).await {
        if let Ok(remote_id) = apis::google_calendar::publish(&event, &token).await {
            event.remote_id = Some(remote_id);
            event.dirty = Some(false);
        }
    }

    db::create_event(&pool, event)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_event(app: tauri::AppHandle, mut event: domain::AppEvent) -> Result<(), String> {
    let pool = app
        .try_state::<sqlx::SqlitePool>()
        .ok_or("Database not initialized yet")?;
        
    event.dirty = Some(true);

    if let Ok(token) = auth::get_valid_access_token(&pool).await {
        if let Ok(remote_id) = apis::google_calendar::publish(&event, &token).await {
            event.remote_id = Some(remote_id);
            event.dirty = Some(false);
        }
    }

    db::update_event(&pool, event)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_event(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let pool = app
        .try_state::<sqlx::SqlitePool>()
        .ok_or("Database not initialized yet")?;
    db::delete_event(&pool, id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn force_sync(app: tauri::AppHandle) -> Result<(), String> {
    let pool = app
        .try_state::<sqlx::SqlitePool>()
        .ok_or("Database not initialized yet")?;
    
    // Attempt sync and emit the same events the background engine does
    let _ = app.emit("sync-started", ());
    if let Err(e) = sync::sync_with_google(&pool).await {
        let err_str = e.to_string();
        let _ = app.emit("sync-error", err_str.clone());
        return Err(err_str);
    }
    let _ = app.emit("sync-finished", ());
    Ok(())
}

#[tauri::command]
fn window_minimize(window: tauri::Window) {
    let _ = window.minimize();
    drop(window);
}

#[tauri::command]
fn window_maximize(window: tauri::Window) {
    if matches!(window.is_maximized(), Ok(true)) {
        let _ = window.unmaximize();
    } else {
        let _ = window.maximize();
    }
    drop(window);
}

#[tauri::command]
fn window_close(window: tauri::Window) {
    let _ = window.close();
    drop(window);
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn window_restore_main(app: tauri::AppHandle) {
    if let Some(main_win) = app.get_webview_window("main") {
        let _ = main_win.show();
        let _ = main_win.unminimize();
        let _ = main_win.set_focus();
    }
    // Automatically hide minitracker when restoring main
    if let Some(mini_win) = app.get_webview_window("minitracker") {
        let _ = mini_win.hide();
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn show_minitracker(app: tauri::AppHandle) {
    if let Some(mini_win) = app.get_webview_window("minitracker") {
        let _ = mini_win.show();
        let _ = mini_win.unminimize();
        let _ = mini_win.set_focus();
    }
    // Also minimize or hide main when showing minitracker
    if let Some(main_win) = app.get_webview_window("main") {
        let _ = main_win.hide();
    }
}

#[tauri::command]
fn hide_launcher(app: tauri::AppHandle) {
    if let Some(launcher) = app.get_webview_window("launcher") {
        let _ = launcher.hide();
    }
    drop(app);
}

#[tauri::command]
fn resize_launcher(app: tauri::AppHandle, height: f64) {
    if let Some(launcher) = app.get_webview_window("launcher") {
        let _ = launcher.set_size(tauri::Size::Logical(tauri::LogicalSize::new(640.0, height)));
    }
    drop(app);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::too_many_lines)]
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
            app.manage(stopwatch::StopwatchManager(std::sync::Mutex::new(stopwatch::StopwatchState::default())));
            let handle = app.handle().clone();

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

            tauri::async_runtime::spawn(async move {
                if let Ok(app_data_dir) = handle.path().app_data_dir() {
                    if std::fs::create_dir_all(&app_data_dir).is_ok() {
                        let db_path = app_data_dir.join("taskroot.db");
                        if let Some(db_path_str) = db_path.to_str() {
                            let db_path_str = format!("sqlite:{db_path_str}");
                            match db::init_db(&db_path_str).await {
                                Ok(pool) => {
                                    handle.manage(pool.clone());
                                    sync::start_sync_engine(handle.clone(), pool);
                                }
                                Err(e) => {
                                    eprintln!("Failed to initialize DB: {e}");
                                }
                            }
                        }
                    }
                }
            });
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
            parse_sigils,
            get_tasks,
            get_events,
            create_task,
            update_task,
            delete_task,
            create_event,
            update_event,
            delete_event,
            force_sync,
            screens::plan::get_plan_layout,
            screens::plan::get_filtered_tasks,
            window_minimize,
            window_maximize,
            window_close,
            window_restore_main,
            show_minitracker,
            hide_launcher,
            resize_launcher,
            auth::login_with_google,
            auth::is_logged_in,
            stopwatch::get_stopwatch_state,
            stopwatch::toggle_stopwatch,
            stopwatch::reset_stopwatch,
            stopwatch::set_stopwatch_state,
            settings::get_settings_schema,
            settings::get_settings,
            settings::update_setting
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| panic!("error while running tauri application: {e}"));
}
