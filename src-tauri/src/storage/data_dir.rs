//! Tauri-free app data directory resolution shared by desktop and CLI.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::error::AppError;

use super::PERSISTED_FILES;

pub const APP_IDENTIFIER: &str = "com.arkma.taskroot";
pub const DATA_DIR_ENV: &str = "TASKROOT_DATA_DIR";

/// Resolve the default data directory for `app_identifier`, honoring
/// `TASKROOT_DATA_DIR` and falling back to `<exe dir>/dev-data` in debug builds
/// when the preferred directory is not writable.
///
/// # Errors
///
/// Returns [`AppError::Store`] if no platform base directory can be located, or
/// [`AppError::Internal`] if the directory cannot be created.
pub fn resolve_default_data_dir(app_identifier: &str) -> Result<PathBuf, AppError> {
    let configured = non_empty_env(DATA_DIR_ENV);
    let preferred = match &configured {
        Some(value) => PathBuf::from(value),
        None => platform_app_data_dir(app_identifier)?,
    };
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    resolve_data_dir(preferred, configured.is_some(), exe_dir.as_deref())
}

/// Create `preferred` or fall back to `<exe_dir>/dev-data` in debug builds when
/// `preferred` is not explicitly configured and cannot be written.
///
/// # Errors
///
/// Returns [`AppError::Internal`] if neither `preferred` nor a debug fallback
/// directory can be created.
pub fn resolve_data_dir(
    preferred: PathBuf,
    explicit: bool,
    exe_dir: Option<&Path>,
) -> Result<PathBuf, AppError> {
    match std::fs::create_dir_all(&preferred) {
        Ok(()) if explicit || !cfg!(debug_assertions) || is_writable(&preferred) => Ok(preferred),
        Ok(()) => Ok(exe_dir
            .and_then(|dir| dev_data_fallback(&preferred, dir))
            .unwrap_or(preferred)),
        Err(error) => {
            if !explicit && cfg!(debug_assertions) {
                if let Some(fallback) = exe_dir.and_then(|dir| dev_data_fallback(&preferred, dir)) {
                    return Ok(fallback);
                }
            }
            Err(AppError::Internal(format!(
                "Failed to create app data dir: {error}"
            )))
        }
    }
}

/// Probe whether `dir` accepts new files.
#[must_use]
pub fn is_writable(dir: &Path) -> bool {
    let probe = dir.join(".write-probe");
    let written = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(&probe);
    written
}

fn non_empty_env(key: &str) -> Option<std::ffi::OsString> {
    std::env::var_os(key).filter(|value| !value.is_empty())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Platform {
    Windows,
    MacOs,
    Unix,
}

const fn current_platform() -> Platform {
    if cfg!(target_os = "windows") {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOs
    } else {
        Platform::Unix
    }
}

fn platform_app_data_dir(app_identifier: &str) -> Result<PathBuf, AppError> {
    let appdata = non_empty_env("APPDATA");
    let xdg = non_empty_env("XDG_DATA_HOME");
    let home = non_empty_env("HOME");
    platform_app_data_dir_from(
        current_platform(),
        app_identifier,
        appdata.as_deref(),
        xdg.as_deref(),
        home.as_deref(),
    )
}

fn platform_app_data_dir_from(
    platform: Platform,
    app_identifier: &str,
    appdata: Option<&OsStr>,
    xdg_data_home: Option<&OsStr>,
    home: Option<&OsStr>,
) -> Result<PathBuf, AppError> {
    let base = match platform {
        Platform::Windows => appdata.map(PathBuf::from),
        Platform::MacOs => home.map(|dir| {
            PathBuf::from(dir)
                .join("Library")
                .join("Application Support")
        }),
        Platform::Unix => xdg_data_home
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute())
            .or_else(|| home.map(|dir| PathBuf::from(dir).join(".local").join("share"))),
    };
    base.map(|dir| dir.join(app_identifier)).ok_or_else(|| {
        AppError::Store(format!(
            "cannot locate the app data directory; set {DATA_DIR_ENV}"
        ))
    })
}

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
    for name in PERSISTED_FILES {
        let file = target.join(name);
        if !file.exists() {
            let _ = std::fs::copy(source.join(name), &file);
        }
    }
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
    fn platform_app_data_dir_matches_windows_macos_and_xdg_linux() {
        let win = platform_app_data_dir_from(
            Platform::Windows,
            APP_IDENTIFIER,
            Some(OsStr::new(r"C:\Users\alice\AppData\Roaming")),
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            win,
            PathBuf::from(r"C:\Users\alice\AppData\Roaming").join(APP_IDENTIFIER)
        );

        let mac = platform_app_data_dir_from(
            Platform::MacOs,
            APP_IDENTIFIER,
            None,
            None,
            Some(OsStr::new("/Users/alice")),
        )
        .unwrap();
        assert_eq!(
            mac,
            PathBuf::from("/Users/alice")
                .join("Library")
                .join("Application Support")
                .join(APP_IDENTIFIER)
        );

        let xdg_root = scratch_dir("xdg-root");
        let linux_xdg = platform_app_data_dir_from(
            Platform::Unix,
            APP_IDENTIFIER,
            None,
            Some(xdg_root.as_os_str()),
            Some(OsStr::new("/home/alice")),
        )
        .unwrap();
        assert_eq!(linux_xdg, xdg_root.join(APP_IDENTIFIER));

        // Relative XDG_DATA_HOME is ignored per the XDG Base Directory spec.
        let linux_fallback = platform_app_data_dir_from(
            Platform::Unix,
            APP_IDENTIFIER,
            None,
            Some(OsStr::new("relative/xdg")),
            Some(OsStr::new("/home/alice")),
        )
        .unwrap();
        assert_eq!(
            linux_fallback,
            PathBuf::from("/home/alice")
                .join(".local")
                .join("share")
                .join(APP_IDENTIFIER)
        );

        let missing = platform_app_data_dir_from(Platform::Unix, APP_IDENTIFIER, None, None, None)
            .unwrap_err();
        assert_eq!(missing.code(), "store");
    }

    #[test]
    fn a_writable_data_dir_is_used_as_is() {
        let dir = scratch_dir("writable");
        assert_eq!(dev_data_fallback(&dir, &dir), None);
    }

    #[test]
    fn an_unwritable_data_dir_falls_back_next_to_the_binary() {
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
    fn seeding_copies_all_persisted_files_without_overwriting() {
        let root = scratch_dir("seed");
        let source = root.join("source");
        let target = root.join("target");
        std::fs::create_dir_all(&source).expect("create source");
        std::fs::create_dir_all(&target).expect("create target");
        let fixtures = [
            ("account.json", "{\"server\":\"new\"}"),
            ("settings.json", "{\"stale\":true}"),
            ("ui-state.json", "{\"tab\":\"plan\"}"),
            ("stopwatch-focus.json", "{\"taskId\":\"t1\"}"),
            ("stopwatch-history.json", "[{\"kind\":\"work\"}]"),
        ];
        for (name, body) in fixtures {
            std::fs::write(source.join(name), body).expect("write source fixture");
        }
        std::fs::write(target.join("settings.json"), "{\"kept\":true}")
            .expect("write target settings");

        seed_from(&source, &target);

        for (name, body) in fixtures {
            let expected = if name == "settings.json" {
                "{\"kept\":true}"
            } else {
                body
            };
            assert_eq!(
                std::fs::read_to_string(target.join(name)).expect("seeded file"),
                expected
            );
        }
    }

    #[test]
    fn a_preferred_dir_that_cannot_be_created_falls_back_in_debug() {
        let root = scratch_dir("create-fail");
        let blocker = root.join("file-blocker");
        std::fs::write(&blocker, b"").expect("create blocking file");
        let preferred = blocker.join("child");
        let exe_dir = root.join("bin");
        std::fs::create_dir_all(&exe_dir).expect("create fake exe dir");

        assert_eq!(
            resolve_data_dir(preferred.clone(), false, Some(&exe_dir)).expect("fallback"),
            exe_dir.join("dev-data")
        );
        assert!(resolve_data_dir(preferred, true, Some(&exe_dir)).is_err());
    }
}
