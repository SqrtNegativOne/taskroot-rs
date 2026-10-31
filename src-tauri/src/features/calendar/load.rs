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
use super::store::{Store, ops};

/// Re-read the account's collections and replace the in-memory resources.
///
/// # Errors
///
/// Returns the account, transport or parse failure. The previous resources stay
/// in place when the fetch fails, because `ops::full_load` only swaps them after
/// every collection has been fetched.
pub async fn load_into(
    store: &mut Store,
    client: &CalDavClient,
    dir: &Path,
) -> Result<(), AppError> {
    let mut account = account::read_account(dir)?
        .ok_or_else(|| AppError::Auth("No CalDAV account is configured".to_string()))?;
    let calendars = discovery::refresh_collections(dir, client, &mut account).await;

    store.calendars = calendars;
    ops::full_load(store, client).await
}
