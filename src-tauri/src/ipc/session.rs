//! Shared Tauri plumbing for commands that touch the `CalDAV` account or store.

use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::AppError;
use crate::features::calendar::{
    CalDavClient, HttpClient, StoreState, connect, load_into,
};

pub(crate) use crate::features::calendar::app_calendar;

/// The process-wide HTTP client plus the stored account credentials.
///
/// # Errors
///
/// Returns [`AppError::Auth`] when no account/password is available and
/// [`AppError::Internal`] when the HTTP client was never initialized.
pub(crate) async fn client_for(app: &AppHandle) -> Result<CalDavClient, AppError> {
    let dir = crate::platform::app_data_dir(app)?;
    let http = app
        .try_state::<HttpClient>()
        .ok_or_else(|| AppError::NotReady("The HTTP client is not initialized".to_string()))?
        .inner()
        .clone();
    let (_account, client) = connect::connect(&dir, &http, None)?;
    Ok(client)
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
/// # Errors
///
/// Returns the account/transport/parse failure from the load.
pub(crate) async fn load_all(app: &AppHandle) -> Result<(), AppError> {
    let client = client_for(app).await?;
    let dir = crate::platform::app_data_dir(app)?;
    let state = store_state(app)?;

    let mut store = state.lock().await;
    load_into(&mut store, &client, &dir).await?;
    drop(store);

    let _ = app.emit(crate::ipc_events::STORE_UPDATED, ());
    Ok(())
}

/// Start the one-shot startup load.
pub(crate) fn start_load(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = load_all(&app).await {
            eprintln!("Startup load failed: {error}");
        }
    });
}
