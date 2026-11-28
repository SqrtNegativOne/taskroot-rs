//! App data directory and `WebView2` storage setup for the desktop shell.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::storage;

static DATA_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

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
pub fn app_data_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    if let Some(dir) = DATA_DIR.get() {
        return Ok(dir.clone());
    }
    let configured = std::env::var_os(storage::DATA_DIR_ENV).filter(|value| !value.is_empty());
    let preferred = match &configured {
        Some(value) => PathBuf::from(value),
        None => app
            .path()
            .app_data_dir()
            .map_err(|e| AppError::Internal(format!("Failed to resolve app data dir: {e}")))?,
    };
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let dir = storage::resolve_data_dir(preferred, configured.is_some(), exe_dir.as_deref())?;
    Ok(DATA_DIR.get_or_init(|| dir).clone())
}

/// Point `WebView2`'s profile beside the executable when the real one is
/// unwritable.
///
/// It keeps its profile in `%LOCALAPPDATA%\<identifier>` and creates no window
/// at all when that directory cannot be written, which leaves a sandboxed dev
/// run with no UI.
pub fn configure_dev_webview_storage(context: &tauri::Context) {
    if !cfg!(debug_assertions) || std::env::var_os("WEBVIEW2_USER_DATA_FOLDER").is_some() {
        return;
    }
    let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) else {
        return;
    };
    let preferred = local.join(context.config().identifier.as_str());
    if std::fs::create_dir_all(&preferred).is_ok() && storage::is_writable(&preferred) {
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(dir) = exe
        .parent()
        .map(|dir| dir.join("dev-data").join("EBWebView"))
    else {
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

/// Configure Chromium / `WebView2` arguments to prevent background throttling
/// and renderer suspension when windows are inactive or when the display sleeps.
pub fn configure_webview_flags() {
    #[cfg(target_os = "windows")]
    {
        const FLAGS: &str = "--disable-background-timer-throttling --disable-renderer-backgrounding";
        match std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS") {
            Ok(existing) => {
                if !existing.contains("--disable-background-timer-throttling") {
                    std::env::set_var(
                        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
                        format!("{existing} {FLAGS}"),
                    );
                }
            }
            Err(_) => {
                std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", FLAGS);
            }
        }
    }
}
