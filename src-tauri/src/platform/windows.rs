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
    set_minitracker_visible(app, true);
}

pub fn set_minitracker_visible(app: &AppHandle, visible: bool) {
    let Some(mini_win) = app.get_webview_window(WindowLabel::Minitracker.as_str()) else {
        return;
    };
    if !visible {
        let _ = mini_win.hide();
        return;
    }
    let _ = mini_win.show();
    let _ = mini_win.unminimize();
    let _ = mini_win.set_focus();
    let _ = mini_win.set_always_on_top(true);
    #[cfg(target_os = "windows")]
    {
        square_window_corners(&mini_win);
        force_topmost(&mini_win);
    }
}

/// Keeps the visible mini tracker above the Windows taskbar.
///
/// Windows keeps the taskbar above ordinary topmost windows after it is
/// activated, so the guard reasserts the bar's place at the top of the topmost
/// band. The screen-effect overlay is left above it.
pub fn start_minitracker_topmost_guard(app: &AppHandle) {
    #[cfg(target_os = "windows")]
    {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(750)).await;
                keep_minitracker_on_top(&app);
            }
        });
    }
    #[cfg(not(target_os = "windows"))]
    let _ = app;
}

#[cfg(target_os = "windows")]
fn keep_minitracker_on_top(app: &AppHandle) {
    let effect_visible = app
        .get_webview_window(WindowLabel::Effect.as_str())
        .is_some_and(|window| matches!(window.is_visible(), Ok(true)));
    if effect_visible {
        return;
    }

    let Some(mini_win) = app.get_webview_window(WindowLabel::Minitracker.as_str()) else {
        return;
    };
    if matches!(mini_win.is_visible(), Ok(true)) {
        force_topmost(&mini_win);
    }
}

#[cfg(target_os = "windows")]
fn force_topmost(window: &tauri::WebviewWindow) {
    use windows::Win32::UI::WindowsAndMessaging::{
        HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    };

    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    // SAFETY: `hwnd` is a live window owned by Tauri. The flags keep its size,
    // position and focus unchanged; only the z-order is raised.
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

#[cfg(target_os = "windows")]
fn square_window_corners(window: &tauri::WebviewWindow) {
    use windows::Win32::Graphics::Dwm::{
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DwmSetWindowAttribute,
    };

    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    let preference = DWMWCP_DONOTROUND;
    let size = u32::try_from(std::mem::size_of_val(&preference)).unwrap_or_default();
    if size == 0 {
        return;
    }
    // SAFETY: `hwnd` is live and `preference` outlives the call; Windows either
    // applies the preference or returns an error on older Windows versions.
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::from_ref(&preference).cast(),
            size,
        );
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
