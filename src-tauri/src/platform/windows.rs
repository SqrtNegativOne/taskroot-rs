//! Window labels and window lifecycle helpers.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use ts_rs::TS;

use super::window_lifecycle::WindowLifecycle;

static MINITRACKER_LIFECYCLE: WindowLifecycle = WindowLifecycle::new();
static LAUNCHER_LIFECYCLE: WindowLifecycle = WindowLifecycle::new();
static EFFECT_LIFECYCLE: WindowLifecycle = WindowLifecycle::new();

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

/// Return the window, building it from its `tauri.conf.json` entry on first use.
///
/// The launcher, mini tracker and effect windows are declared with
/// `"create": false` so starting the app does not pay for three extra webviews
/// before the main window appears.
async fn ensure_window(app: &AppHandle, label: &WindowLabel) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(label.as_str()) {
        return Ok(window);
    }
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == label.as_str())
        .ok_or(tauri::Error::WindowNotFound)?
        .clone();
    let app = app.clone();
    super::window_creation::create_off_thread(move || {
        if let Some(window) = app.get_webview_window(&config.label) {
            return Ok(window);
        }
        WebviewWindowBuilder::from_config(&app, &config)?.build()
    })
    .await
}

pub fn restore_main(app: &AppHandle) {
    let Some(main_win) = app.get_webview_window(WindowLabel::Main.as_str()) else {
        return;
    };
    let _ = main_win.show();
    let _ = main_win.unminimize();
    let _ = main_win.set_focus();
}

/// # Errors
///
/// Returns an error if the window cannot be created or its visibility changed.
pub async fn set_minitracker_visible(app: &AppHandle, visible: bool) -> tauri::Result<()> {
    let _operation = MINITRACKER_LIFECYCLE.lock().await;
    if !visible {
        if let Some(mini_win) = app.get_webview_window(WindowLabel::Minitracker.as_str()) {
            mini_win.hide()?;
        }
        return Ok(());
    }
    let mini_win = ensure_window(app, &WindowLabel::Minitracker).await?;
    mini_win.show()?;
    mini_win.unminimize()?;
    mini_win.set_focus()?;
    mini_win.set_always_on_top(true)?;
    #[cfg(target_os = "windows")]
    {
        square_window_corners(&mini_win);
        force_topmost(&mini_win);
    }
    Ok(())
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

pub async fn hide_launcher(app: &AppHandle) {
    let _operation = LAUNCHER_LIFECYCLE.lock().await;
    if let Some(launcher_win) = app.get_webview_window(WindowLabel::Launcher.as_str()) {
        let _ = launcher_win.hide();
    }
}

/// Show the full-screen, click-through phase-transition effect overlay.
///
/// # Errors
///
/// Returns an error if the overlay cannot be created or shown.
pub async fn show_screen_effect(app: &AppHandle) -> tauri::Result<()> {
    let _operation = EFFECT_LIFECYCLE.lock().await;
    let effect_win = ensure_window(app, &WindowLabel::Effect).await?;
    effect_win.set_ignore_cursor_events(true)?;
    effect_win.set_always_on_top(true)?;
    effect_win.show()
}

pub async fn hide_screen_effect(app: &AppHandle) {
    let _operation = EFFECT_LIFECYCLE.lock().await;
    if let Some(effect_win) = app.get_webview_window(WindowLabel::Effect.as_str()) {
        let _ = effect_win.hide();
    }
}

pub fn quit_app(app: &AppHandle) {
    app.exit(0);
}

/// # Errors
///
/// Returns an error if the launcher cannot be created or its visibility changed.
pub async fn toggle_launcher_window(app: &AppHandle) -> tauri::Result<()> {
    let _operation = LAUNCHER_LIFECYCLE.lock().await;
    let launcher_win = ensure_window(app, &WindowLabel::Launcher).await?;

    if matches!(launcher_win.is_visible(), Ok(true)) {
        return launcher_win.hide();
    }

    launcher_win.center()?;
    launcher_win.show()?;
    launcher_win.set_focus()
}

pub fn resize_launcher(app: &AppHandle, width: f64, height: f64) {
    if let Some(launcher_win) = app.get_webview_window(WindowLabel::Launcher.as_str()) {
        let _ = launcher_win.set_size(tauri::Size::Logical(tauri::LogicalSize::new(width, height)));
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
