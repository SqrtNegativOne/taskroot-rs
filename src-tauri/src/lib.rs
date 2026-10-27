#![warn(clippy::pedantic, clippy::nursery)]
#![warn(unreachable_pub)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use std::path::{Path, PathBuf};

use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

pub mod caldav;
pub mod commands;
pub mod domain;
pub mod error;
pub mod events;
pub mod hooks;
pub mod ics;
pub mod reminders;
pub mod screens;
pub mod settings;
pub mod stopwatch;
pub mod store;
pub mod sync;
#[cfg(test)]
pub mod test_support;
pub mod time_utils;

use error::AppError;

pub const LAUNCHER_SHORTCUT: &str = "CommandOrControl+Shift+Space";

static DATA_DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

pub fn toggle_launcher(app: &tauri::AppHandle) {
    commands::window::toggle_launcher_window(app);
}

#[must_use]
pub fn extract_deep_link_route(arg: &str) -> Option<&str> {
    let route = arg.strip_prefix("taskroot://")?;
    Some(route.trim_end_matches('/'))
}

/// The application data directory, created on demand.
///
/// `TASKROOT_DATA_DIR` always wins. Otherwise the platform app-data directory is
/// used, except in a debug build that cannot write to it: a sandboxed dev run
/// keeps its data in `<exe dir>/dev-data` instead, seeded from the real directory
/// so the run still starts with the configured account and settings.
///
/// # Errors
///
/// Returns an error if the path cannot be resolved or created.
pub fn app_data_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, AppError> {
    if let Some(dir) = DATA_DIR.get() {
        return Ok(dir.clone());
    }
    let configured = std::env::var_os("TASKROOT_DATA_DIR");
    let preferred = match &configured {
        Some(value) => PathBuf::from(value),
        None => app
            .path()
            .app_data_dir()
            .map_err(|e| AppError::Internal(format!("Failed to resolve app data dir: {e}")))?,
    };
    std::fs::create_dir_all(&preferred)
        .map_err(|e| AppError::Internal(format!("Failed to create app data dir: {e}")))?;
    let dir = if configured.is_some() {
        preferred
    } else {
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
            .and_then(|exe_dir| dev_data_fallback(&preferred, &exe_dir))
            .unwrap_or(preferred)
    };
    Ok(DATA_DIR.get_or_init(|| dir).clone())
}

/// A sandboxed dev run can read the real app-data directory but not write it, so
/// the store would have nowhere to persist. Keep the run usable by moving its
/// data next to the binary instead, seeded with the files that describe the
/// account.
fn dev_data_fallback(preferred: &Path, exe_dir: &Path) -> Option<PathBuf> {
    if !cfg!(debug_assertions) || is_writable(preferred) {
        return None;
    }
    let dir = exe_dir.join("dev-data");
    std::fs::create_dir_all(&dir).ok()?;
    if !is_writable(&dir) {
        return None;
    }
    seed_from(preferred, &dir);
    eprintln!(
        "App data directory {} is not writable; this debug build uses {}",
        preferred.display(),
        dir.display()
    );
    Some(dir)
}

fn seed_from(source: &Path, target: &Path) {
    for name in ["account.json", "settings.json", "ui-state.json"] {
        let file = target.join(name);
        if !file.exists() {
            let _ = std::fs::copy(source.join(name), &file);
        }
    }
}

fn is_writable(dir: &Path) -> bool {
    let probe = dir.join(".write-probe");
    let written = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(&probe);
    written
}

/// `WebView2` keeps its profile in `%LOCALAPPDATA%\<identifier>` and creates no
/// window at all when that directory cannot be written, which leaves a sandboxed
/// dev run with no UI. Point it beside the executable instead, like `dev-data`.
fn configure_dev_webview_storage(context: &tauri::Context) {
    if !cfg!(debug_assertions) || std::env::var_os("WEBVIEW2_USER_DATA_FOLDER").is_some() {
        return;
    }
    let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) else {
        return;
    };
    let preferred = local.join(context.config().identifier.as_str());
    if std::fs::create_dir_all(&preferred).is_ok() && is_writable(&preferred) {
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(dir) = exe.parent().map(|dir| dir.join("dev-data").join("EBWebView")) else {
        return;
    };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", &dir);
    eprintln!(
        "WebView2 profile directory {} is not writable; this debug build uses {}",
        preferred.display(),
        dir.display()
    );
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(error) = try_run() {
        eprintln!("error while running tauri application: {error}");
    }
}

// `generate_context!` expands to Tauri's own config loader, which terminates the
// process on a malformed manifest. The stack frame belongs to that macro, not to
// a function we can split.
#[allow(clippy::exit, clippy::large_stack_frames)]
fn app_context() -> tauri::Context {
    tauri::generate_context!()
}

fn try_run() -> tauri::Result<()> {
    let context = app_context();
    configure_dev_webview_storage(&context);
    build_app().run(context)
}

fn build_app() -> tauri::Builder<tauri::Wry> {
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
        .setup(setup_app)
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .on_window_event(handle_window_event)
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
            screens::plan::query_tasks,
            screens::plan::query_events,
            screens::plan::query_event_instances,
            screens::plan::get_event,
            screens::plan::reschedule_event,
            screens::plan::get_task_schema,
            screens::plan::get_event_schema,
            commands::window::window_restore_main,
            commands::window::show_minitracker,
            commands::window::hide_launcher,
            commands::window::show_screen_effect,
            commands::window::hide_screen_effect,
            commands::window::quit_app,
            commands::window::toggle_launcher,
            commands::window::resize_launcher,
            commands::stopwatch::get_stopwatch_state,
            commands::stopwatch::toggle_stopwatch,
            commands::stopwatch::toggle_break,
            commands::stopwatch::toggle_pause,
            commands::stopwatch::adjust_pause,
            commands::stopwatch::reset_stopwatch,
            settings::get_settings_schema,
            settings::get_settings,
            settings::get_ui_state,
            settings::set_ui_state,
            settings::update_setting,
            commands::sync::get_sync_state,
            caldav::commands::is_account_configured,
            caldav::commands::get_account,
            caldav::commands::test_account,
            caldav::commands::connect_account,
            caldav::commands::disconnect_account,
            caldav::commands::create_calendar,
            caldav::commands::rename_calendar,
            caldav::commands::set_calendar_color,
            caldav::commands::delete_calendar
        ])
}

fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let _ = app.global_shortcut().register(LAUNCHER_SHORTCUT);
    app.manage(stopwatch::StopwatchManager(std::sync::Mutex::new(
        stopwatch::StopwatchState::default(),
    )));
    app.manage(sync::SyncStateManager(std::sync::Mutex::new(
        sync::SyncState::default(),
    )));
    app.manage(reminders::ReminderState::default());
    app.manage(caldav::transport::HttpClient::new()?);
    let handle = app.handle().clone();

    if let Some(effect_win) = app.get_webview_window(domain::WindowLabel::Effect.as_str()) {
        let _ = effect_win.set_ignore_cursor_events(true);
    }

    let open_i = tauri::menu::MenuItem::with_id(app, "open", "Open Taskroot", true, None::<&str>)?;
    let exit_i = tauri::menu::MenuItem::with_id(app, "exit", "Exit", true, None::<&str>)?;
    let menu = tauri::menu::Menu::with_items(app, &[&open_i, &exit_i])?;

    let mut tray_builder =
        tauri::tray::TrayIconBuilder::new().menu(&menu).show_menu_on_left_click(false);

    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    let _tray = tray_builder
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Some(main_win) = app.get_webview_window(domain::WindowLabel::Main.as_str()) {
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
                if let Some(main_win) = app.get_webview_window(domain::WindowLabel::Main.as_str()) {
                    let _ = main_win.show();
                    let _ = main_win.unminimize();
                    let _ = main_win.set_focus();
                }
            }
        })
        .build(app)?;

    let data_dir = app_data_dir(&handle)?;
    let store = store::Store::load(data_dir)?;
    handle.manage(store::StoreState::new(store));
    sync::start_sync_engine(handle.clone());
    hooks::start(handle.clone());
    reminders::start_reminder_engine(handle.clone());

    tauri::async_runtime::spawn(async move {
        if let Err(error) = sync::run_tracked_sync(&handle, None).await {
            eprintln!("Startup load failed: {error}");
        }
    });

    Ok(())
}

fn handle_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        if window.label() == domain::WindowLabel::Main.as_str() {
            let _ = window.hide();
            api.prevent_close();
        }
    }
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

    fn scratch_dir(name: &str) -> PathBuf {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("lib-tests")
            .join(format!("{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    #[test]
    fn a_writable_data_dir_is_used_as_is() {
        let dir = scratch_dir("writable");
        assert_eq!(dev_data_fallback(&dir, &dir), None);
    }

    #[test]
    fn an_unwritable_data_dir_falls_back_next_to_the_binary() {
        // A regular file can never accept the write probe.
        let root = scratch_dir("unwritable");
        let blocked = root.join("blocked");
        std::fs::write(&blocked, b"").expect("create the blocking file");
        let exe_dir = root.join("bin");
        std::fs::create_dir_all(&exe_dir).expect("create the fake binary dir");

        assert_eq!(
            dev_data_fallback(&blocked, &exe_dir),
            Some(exe_dir.join("dev-data"))
        );
    }

    #[test]
    fn seeding_copies_the_account_files_without_overwriting() {
        let root = scratch_dir("seed");
        let source = root.join("source");
        let target = root.join("target");
        std::fs::create_dir_all(&source).expect("create source");
        std::fs::create_dir_all(&target).expect("create target");
        std::fs::write(source.join("account.json"), "{\"server\":\"new\"}").expect("write account");
        std::fs::write(source.join("settings.json"), "{\"stale\":true}").expect("write settings");
        std::fs::write(target.join("settings.json"), "{\"kept\":true}").expect("write target settings");

        seed_from(&source, &target);

        assert_eq!(
            std::fs::read_to_string(target.join("account.json")).expect("copied account"),
            "{\"server\":\"new\"}"
        );
        assert_eq!(
            std::fs::read_to_string(target.join("settings.json")).expect("kept settings"),
            "{\"kept\":true}"
        );
    }
}
