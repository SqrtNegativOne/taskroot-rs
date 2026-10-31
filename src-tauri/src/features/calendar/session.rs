//! A headless calendar session: the store, the client and a one-shot load.
//!
//! This is the calendar API the CLI uses. The Tauri adapters (`ipc::session`)
//! resolve a session from managed state and the app data directory.

use std::path::PathBuf;

use crate::error::AppError;

use super::caldav::client::CalDavClient;
use super::caldav::connect;
use super::caldav::discovery;
use super::caldav::model::CalendarInfo;
use super::caldav::transport::HttpClient;
use super::domain::{AppCalendar, CollectionId};
use super::store::{Store, ops};

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
pub async fn open(dir: PathBuf) -> Result<Session, AppError> {
    let mut store = Store::load(dir.clone())?;
    let http = HttpClient::new()?;
    let (mut account, client) = connect::connect(&dir, &http, None)?;
    let calendars = discovery::refresh_collections(&dir, &client, &mut account).await;
    store.calendars.clone_from(&calendars);
    ops::full_load(&mut store, &client).await?;
    Ok(Session { dir, store, client })
}

/// Project a discovered collection onto the view DTO the frontend consumes.
#[must_use]
pub fn app_calendar(info: &CalendarInfo) -> AppCalendar {
    AppCalendar {
        id: CollectionId::from(info.href.clone()),
        summary: info.display_name.clone(),
        color: info
            .color
            .as_deref()
            .and_then(|color| crate::features::calendar::domain::Color::try_from(color.to_string()).ok()),
        writable: info.writable,
        component: info.component,
    }
}
