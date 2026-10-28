//! Load the `CalDAV` collections into the in-memory store.
//!
//! There is no background sync: the app loads once at startup, when an account
//! is connected, and when the user presses Refresh. Every edit still `PUT`s to
//! Radicale immediately.

use tauri::{AppHandle, Emitter};

use crate::commands::session::{client_for, store_state};
use crate::error::AppError;
use crate::store::ops;

/// Re-read the account's collections and replace the in-memory resources.
///
/// # Errors
///
/// Returns the account, transport or parse failure. The previous resources stay
/// in place when the fetch fails, because `ops::full_load` only swaps them after
/// every collection has been fetched.
pub async fn load_all(app: &AppHandle) -> Result<(), AppError> {
    let client = client_for(app).await?;
    let dir = crate::app_data_dir(app)?;
    let mut account = crate::caldav::account::read_account(&dir)?
        .ok_or_else(|| AppError::Auth("No CalDAV account is configured".to_string()))?;
    let calendars =
        crate::caldav::discovery::refresh_collections(&dir, &client, &mut account).await;

    let state = store_state(app)?;
    let mut store = state.lock().await;
    store.calendars = calendars;
    ops::full_load(&mut store, &client).await?;
    drop(store);

    let _ = app.emit(crate::events::STORE_UPDATED, ());
    Ok(())
}

/// Start the one-shot startup load.
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = load_all(&app).await {
            eprintln!("Startup load failed: {error}");
        }
    });
}
