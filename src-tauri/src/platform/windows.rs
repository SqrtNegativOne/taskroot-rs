//! Window labels and window lifecycle helpers.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WindowEvent};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/lib/bindings/WindowLabel.generated.ts")]
#[serde(rename_all = "kebab-case")]
pub enum WindowLabel {
    Main,
    Launcher,
    Minitracker,
    Effect,
}

impl WindowLabel {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Launcher => "launcher",
            Self::Minitracker => "minitracker",
            Self::Effect => "effect",
        }
    }
}

pub fn restore_main(app: &AppHandle) {
    let Some(main_win) = app.get_webview_window(WindowLabel::Main.as_str()) else {
        return;
    };
    let _ = main_win.show();
    let _ = main_win.unminimize();
    let _ = main_win.set_focus();
}

pub fn show_minitracker(app: &AppHandle) {
    if let Some(mini_win) = app.get_webview_window(WindowLabel::Minitracker.as_str()) {
        let _ = mini_win.show();
        let _ = mini_win.unminimize();
        let _ = mini_win.set_focus();
        let _ = mini_win.set_always_on_top(true);
    }
}

pub fn hide_launcher(app: &AppHandle) {
    if let Some(launcher_win) = app.get_webview_window(WindowLabel::Launcher.as_str()) {
        let _ = launcher_win.hide();
    }
}

/// Show the full-screen, click-through phase-transition effect overlay.
pub fn show_screen_effect(app: &AppHandle) {
    if let Some(effect_win) = app.get_webview_window(WindowLabel::Effect.as_str()) {
        let _ = effect_win.set_always_on_top(true);
        let _ = effect_win.show();
    }
}

pub fn hide_screen_effect(app: &AppHandle) {
    if let Some(effect_win) = app.get_webview_window(WindowLabel::Effect.as_str()) {
        let _ = effect_win.hide();
    }
}

pub fn quit_app(app: &AppHandle) {
    app.exit(0);
}

pub fn toggle_launcher_window(app: &AppHandle) {
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

pub fn resize_launcher(app: &AppHandle, width: f64, height: f64) {
    if let Some(launcher_win) = app.get_webview_window(WindowLabel::Launcher.as_str()) {
        let _ = launcher_win.set_size(tauri::Size::Logical(tauri::LogicalSize::new(width, height)));
    }
}

pub fn configure_effect_window(app: &tauri::App) {
    if let Some(effect_win) = app.get_webview_window(WindowLabel::Effect.as_str()) {
        let _ = effect_win.set_ignore_cursor_events(true);
    }
}

pub fn handle_window_event(window: &tauri::Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        if window.label() == WindowLabel::Main.as_str() {
            let _ = window.hide();
            api.prevent_close();
        }
    }
}
