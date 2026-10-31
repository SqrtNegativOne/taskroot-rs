//! App data directory resolution, including the sandboxed dev fallback.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::error::AppError;

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
    if std::fs::create_dir_all(&preferred).is_ok() && is_writable(&preferred) {
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

#[cfg(test)]
mod tests {
    use super::*;

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
        std::fs::write(target.join("settings.json"), "{\"kept\":true}")
            .expect("write target settings");

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
