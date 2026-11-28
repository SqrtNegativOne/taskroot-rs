//! A headless calendar session: the store, account service, client and load.
//!
//! This is the calendar API the CLI uses. The Tauri adapters (`ipc::session`)
//! resolve a session from managed state and the app data directory.

use std::path::PathBuf;
use std::sync::Arc;

use crate::error::AppError;

use super::caldav::account::{AccountService, JsonAccountRepository, KeychainCredentials};
use super::caldav::client::CalDavClient;
use super::caldav::model::CalendarInfo;
use super::caldav::transport::HttpClient;
use super::domain::{AppCalendar, CollectionId};
use super::store::{Store, StoreState};

pub struct Session {
    pub dir: PathBuf,
    pub accounts: AccountService,
    pub store: StoreState,
    pub client: CalDavClient,
}

/// Load the account, the keychain/env password and every collection via the
/// shared [`AccountService::refresh`] startup path.
///
/// # Errors
///
/// Propagates account, discovery and load failures.
pub async fn open(dir: PathBuf) -> Result<Session, AppError> {
    let store = StoreState::new(Store::load(dir.clone()));
    let http = HttpClient::new()?;
    let accounts = AccountService::new(
        Arc::new(JsonAccountRepository::new(dir.clone())),
        Arc::new(KeychainCredentials),
        http,
    );
    accounts.refresh(&store).await?;
    let client = accounts.client().await?.into_client();
    Ok(Session {
        dir,
        accounts,
        store,
        client,
    })
}

/// Project a discovered collection onto the view DTO the frontend consumes.
#[must_use]
pub fn app_calendar(info: &CalendarInfo) -> AppCalendar {
    AppCalendar {
        id: CollectionId::from(info.href.clone()),
        summary: info.display_name.clone(),
        color: info.color.as_deref().and_then(|color| {
            crate::features::calendar::domain::Color::try_from(color.to_string()).ok()
        }),
        writable: info.writable,
        component: info.component,
    }
}
