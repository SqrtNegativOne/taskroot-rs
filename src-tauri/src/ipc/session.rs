//! Shared Tauri plumbing for commands that touch the `CalDAV` account or store.

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::AppError;
use crate::features::calendar::{account::ClientLease, StoreState};

pub(crate) use crate::features::calendar::app_calendar;

/// How many times the startup load retries a transient transport failure.
///
/// The app is often launched by the desktop session before the local Radicale
/// process has started listening. Without retries that race leaves the store
/// empty until the user presses Refresh.
const STARTUP_LOAD_ATTEMPTS: u32 = 20;
const STARTUP_LOAD_RETRY_DELAY: Duration = Duration::from_secs(2);

/// The process-wide HTTP client plus the stored account credentials.
///
/// # Errors
///
/// Returns [`AppError::Auth`] when no account/password is available and
/// [`AppError::Internal`] when the HTTP client was never initialized.
pub(crate) async fn client_for(app: &AppHandle) -> Result<ClientLease, AppError> {
    super::accounts::service(app)?.client().await
}

/// # Errors
///
/// Returns [`AppError::NotReady`] when the store was never initialized.
pub(crate) fn store_state(app: &AppHandle) -> Result<State<'_, StoreState>, AppError> {
    app.try_state::<StoreState>()
        .ok_or_else(|| AppError::NotReady("The store is not initialized".to_string()))
}

/// Reload every collection, then tell the windows to re-query.
///
/// The network fetch runs without the store lock, so settings reads and
/// queries stay responsive while a load is in flight. The fetched snapshot is
/// only applied when no edit touched the store in the meantime.
///
/// # Errors
///
/// Returns the account/transport/parse failure from the load.
pub(crate) async fn load_all(app: &AppHandle) -> Result<(), AppError> {
    let state = store_state(app)?;
    super::accounts::service(app)?.refresh(&state).await?;

    let _ = app.emit(crate::ipc_events::STORE_UPDATED, ());
    Ok(())
}

/// Start the one-shot startup load, retrying while the server is still starting.
pub(crate) fn start_load(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        for attempt in 1..=STARTUP_LOAD_ATTEMPTS {
            match load_all(&app).await {
                Ok(()) => return,
                Err(error) => {
                    let retry =
                        matches!(&error, AppError::Remote(_)) && attempt < STARTUP_LOAD_ATTEMPTS;
                    if !retry {
                        eprintln!("Startup load failed: {error}");
                        return;
                    }
                    eprintln!("Startup load attempt {attempt} failed, retrying: {error}");
                    tokio::time::sleep(STARTUP_LOAD_RETRY_DELAY).await;
                }
            }
        }
    });
}
