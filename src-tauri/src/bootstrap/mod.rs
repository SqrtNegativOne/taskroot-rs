//! Composition root: build the app, register commands and start runtimes.

use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

use crate::features::{calendar, reminders, stopwatch};
use crate::ipc;
use crate::platform;

pub const LAUNCHER_SHORTCUT: &str = "CommandOrControl+Shift+Space";

#[must_use]
pub fn extract_deep_link_route(arg: &str) -> Option<&str> {
    let route = arg.strip_prefix("taskroot://")?;
    Some(route.trim_end_matches('/'))
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
    platform::configure_dev_webview_storage(&context);
    build_app().run(context)
}

fn build_app() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default()
        .plugin(tauri_plugin_deep_link::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        platform::toggle_launcher_window(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            platform::restore_main(app);
            for arg in &args {
                if let Some(route) = extract_deep_link_route(arg) {
                    let _ = app.emit(crate::ipc_events::DEEP_LINK, route);
                }
            }
        }))
        .setup(setup_app)
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .on_window_event(platform::handle_window_event)
        .invoke_handler(tauri::generate_handler![
            ipc::tasks::parse_sigils,
            ipc::tasks::create_task,
            ipc::tasks::update_task,
            ipc::tasks::delete_task,
            ipc::tasks::get_past_due_task_ids,
            ipc::events::create_event,
            ipc::events::update_event,
            ipc::events::delete_event,
            ipc::events::reschedule_event,
            ipc::events::get_active_calendars,
            ipc::refresh::refresh_data,
            ipc::plan::query_tasks,
            ipc::plan::query_events,
            ipc::plan::query_event_instances,
            ipc::plan::get_event,
            ipc::schema::get_task_schema,
            ipc::schema::get_event_schema,
            ipc::window::window_restore_main,
            ipc::window::show_minitracker,
            ipc::window::hide_launcher,
            ipc::window::show_screen_effect,
            ipc::window::hide_screen_effect,
            ipc::window::quit_app,
            ipc::window::toggle_launcher,
            ipc::window::resize_launcher,
            ipc::stopwatch::get_stopwatch_state,
            ipc::stopwatch::toggle_stopwatch,
            ipc::stopwatch::toggle_break,
            ipc::stopwatch::toggle_pause,
            ipc::stopwatch::adjust_pause,
            ipc::stopwatch::reset_stopwatch,
            ipc::settings::get_settings_schema,
            ipc::settings::get_settings,
            ipc::settings::get_ui_state,
            ipc::settings::set_ui_state,
            ipc::settings::update_setting,
            ipc::calendars::is_account_configured,
            ipc::calendars::get_account,
            ipc::calendars::test_account,
            ipc::calendars::connect_account,
            ipc::calendars::disconnect_account,
            ipc::calendars::create_calendar,
            ipc::calendars::rename_calendar,
            ipc::calendars::set_calendar_color,
            ipc::calendars::delete_calendar
        ])
}

fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let _ = app.global_shortcut().register(LAUNCHER_SHORTCUT);
    app.manage(stopwatch::StopwatchManager(std::sync::Mutex::new(
        stopwatch::StopwatchState::default(),
    )));
    app.manage(reminders::ReminderState::default());
    app.manage(calendar::HttpClient::new()?);
    let handle = app.handle().clone();

    platform::configure_effect_window(app);
    platform::build_tray(app)?;

    let data_dir = platform::app_data_dir(&handle)?;
    let store = calendar::Store::load(data_dir)?;
    handle.manage(calendar::StoreState::new(store));
    ipc::session::start_load(handle.clone());
    reminders::start_reminder_engine(handle);

    Ok(())
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
