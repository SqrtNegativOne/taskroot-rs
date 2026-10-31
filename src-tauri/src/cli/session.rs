//! Data-directory discovery and the one-shot `CalDAV` session load.

use std::path::PathBuf;

use crate::error::AppError;
use crate::features::calendar::{Session, open as open_calendar};

const APP_IDENTIFIER: &str = "com.arkma.tauri-app";

/// Load the account, the keychain password and every collection.
///
/// # Errors
///
/// Propagates account, discovery and load failures.
pub async fn open() -> Result<Session, AppError> {
    open_calendar(data_dir()?).await
}

/// # Errors
///
/// Returns [`AppError::Store`] when no data directory can be located.
pub fn data_dir() -> Result<PathBuf, AppError> {
    if let Some(dir) = std::env::var_os("TASKROOT_DATA_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
        })
        .ok_or_else(|| {
            AppError::Store(
                "cannot locate the app data directory; set TASKROOT_DATA_DIR".to_string(),
            )
        })?;
    Ok(base.join(APP_IDENTIFIER))
}
