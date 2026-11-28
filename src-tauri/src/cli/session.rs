//! Data-directory discovery and the one-shot `CalDAV` session load.

use std::path::PathBuf;

use crate::error::AppError;
use crate::features::calendar::{open as open_calendar, Session};
use crate::storage;

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
/// Returns [`AppError::Store`] or [`AppError::Internal`] when the data
/// directory cannot be located or created.
pub fn data_dir() -> Result<PathBuf, AppError> {
    storage::resolve_default_data_dir(storage::APP_IDENTIFIER)
}
