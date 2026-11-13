//! Load the `CalDAV` collections into the in-memory store.
//!
//! There is no background sync: the app loads once at startup, when an account
//! is connected, and when the user presses Refresh. Every edit still `PUT`s to
//! Radicale immediately.
//!
//! This is the pure half. The Tauri adapter (`ipc::session`) resolves the account
//! directory and the live store, then emits `store-updated` afterwards.

use std::path::Path;

use crate::error::AppError;

use super::caldav::account;
use super::caldav::client::CalDavClient;
use super::caldav::discovery;
use super::caldav::model::CalendarInfo;
use super::store::{Store, ops};
use ops::FetchedCollection;

/// A complete load that has not touched any store yet.
pub struct Fetched {
    pub calendars: Vec<CalendarInfo>,
    pub collections: Vec<FetchedCollection>,
}

/// Read the account, refresh its collections and fetch every resource.
///
/// No store lock is needed, so the caller can keep the UI responsive while the
/// network request is in flight.
///
/// # Errors
///
/// Returns the account, transport or parse failure.
pub async fn fetch_into(dir: &Path, client: &CalDavClient) -> Result<Fetched, AppError> {
    let mut account = account::read_account(dir)?
        .ok_or_else(|| AppError::Auth("No CalDAV account is configured".to_string()))?;
    let calendars = discovery::refresh_collections(dir, client, &mut account).await;
    let collections = ops::fetch_collections(client, &calendars).await?;
    Ok(Fetched {
        calendars,
        collections,
    })
}

/// Replace the store with a fetched snapshot.
pub fn apply(store: &mut Store, fetched: Fetched) {
    store.calendars = fetched.calendars;
    ops::apply_full_load(store, fetched.collections);
}

/// Re-read the account's collections and replace the in-memory resources.
///
/// # Errors
///
/// Returns the account, transport or parse failure. The previous resources stay
/// in place when the fetch fails, because nothing is applied until every
/// collection has been fetched.
pub async fn load_into(
    store: &mut Store,
    client: &CalDavClient,
    dir: &Path,
) -> Result<(), AppError> {
    let fetched = fetch_into(dir, client).await?;
    apply(store, fetched);
    Ok(())
}
