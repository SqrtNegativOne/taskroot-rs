// Tauri's IPC contract injects `AppHandle` by value and owns the deserialized
// command arguments, so clippy's borrow suggestion cannot be applied to a
// command wrapper. Keep the exception at the IPC boundary instead of on every
// handler.
#![allow(clippy::needless_pass_by_value)]

pub mod accounts;
pub mod calendars;
pub mod events;
pub mod plan;
pub mod schema;
pub mod session;
pub mod settings;
pub mod stopwatch;
pub mod tasks;
pub mod window;

use tauri_specta::{collect_commands, collect_events, Builder};

use crate::error::{AppError, BackendErrorCode};
use crate::features::stopwatch::history::{StopwatchHistoryEntry, StopwatchHistoryKind};
use crate::ipc_events::{
    AccountChanged, LauncherNavigate, ScreenEffect, SettingsUpdated, StopwatchUpdated, StoreUpdated,
};
use crate::platform::WindowLabel;

/// Single source of truth for Tauri commands, typed events, and TypeScript
/// bindings exported by `tauri-specta`.
#[must_use]
pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            tasks::create_task,
            tasks::update_task,
            tasks::toggle_task,
            tasks::delete_task,
            tasks::get_past_due_task_ids,
            events::create_event,
            events::update_event,
            events::delete_event,
            events::reschedule_event,
            events::get_active_calendars,
            accounts::refresh_data,
            plan::query_tasks,
            plan::query_events,
            plan::query_event_instances,
            plan::query_task_dates,
            plan::list_tags,
            plan::rename_tag,
            schema::get_task_schema,
            window::window_restore_main,
            window::set_minitracker_visible,
            window::hide_launcher,
            window::show_screen_effect,
            window::hide_screen_effect,
            window::resize_launcher,
            stopwatch::get_stopwatch_state,
            stopwatch::get_stopwatch_history,
            stopwatch::toggle_stopwatch,
            stopwatch::toggle_break,
            stopwatch::complete_break,
            stopwatch::toggle_pause,
            stopwatch::adjust_pause,
            stopwatch::set_stopwatch_label,
            stopwatch::set_stopwatch_task,
            stopwatch::reset_stopwatch,
            settings::get_settings_schema,
            settings::get_settings,
            settings::get_ui_state,
            settings::set_ui_state,
            settings::update_setting,
            accounts::get_accounts,
            accounts::test_account,
            accounts::connect_account,
            accounts::switch_account,
            accounts::disconnect_account,
            accounts::remove_account,
            calendars::create_calendar,
            calendars::rename_calendar,
            calendars::set_calendar_color,
            calendars::delete_calendar
        ])
        .events(collect_events![
            StopwatchUpdated,
            AccountChanged,
            StoreUpdated,
            SettingsUpdated,
            ScreenEffect,
            LauncherNavigate
        ])
        .typ::<WindowLabel>()
        .typ::<StopwatchHistoryEntry>()
        .typ::<StopwatchHistoryKind>()
        .typ::<AppError>()
        .typ::<BackendErrorCode>()
        .dangerously_cast_bigints_to_number()
        .disable_serde_phases()
}
