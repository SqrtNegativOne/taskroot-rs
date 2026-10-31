//! Data-directory discovery and the one-shot `CalDAV` session load.

use std::path::PathBuf;

use crate::caldav::client::CalDavClient;
use crate::caldav::connect;
use crate::caldav::discovery;
use crate::caldav::transport::HttpClient;
use crate::error::AppError;
use crate::store::{ops, Store};

const APP_IDENTIFIER: &str = "com.arkma.tauri-app";

pub struct Session {
    pub dir: PathBuf,
    pub store: Store,
    pub client: CalDavClient,
}

/// Load the account, the keychain password and every collection.
///
/// # Errors
///
/// Propagates account, discovery and load failures.
pub async fn open() -> Result<Session, AppError> {
    let dir = data_dir()?;
    let mut store = Store::load(dir.clone())?;
    let http = HttpClient::new()?;
    let (mut account, client) = connect::connect(&dir, &http, None)?;
    let calendars = discovery::refresh_collections(&dir, &client, &mut account).await;
    store.calendars.clone_from(&calendars);
    ops::full_load(&mut store, &client).await?;
    Ok(Session { dir, store, client })
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
