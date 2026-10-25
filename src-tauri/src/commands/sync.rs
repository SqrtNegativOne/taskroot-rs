use crate::commands::session::store_state;
use crate::error::AppError;
use crate::sync;

/// # Errors
///
/// Returns [`AppError::Sync`] when the `CalDAV` round fails.
#[tauri::command]
pub async fn force_sync(app: tauri::AppHandle) -> Result<(), AppError> {
    let next_sync = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::seconds(sync::SYNC_INTERVAL_SECS))
        .unwrap_or_else(chrono::Utc::now);
    sync::run_tracked_sync(&app, Some(next_sync))
        .await
        .map_err(AppError::Sync)
}

/// Forget every in-memory resource and reload from the server.
///
/// # Errors
///
/// Returns an error if the snapshot cannot be written or the reload fails.
#[tauri::command]
pub async fn wipe_local_data(app: tauri::AppHandle) -> Result<(), AppError> {
    let state = store_state(&app)?;
    {
        let mut store = state.lock().await;
        store.resources.clear();
        store.tokens.clear();
        store.write_snapshot()?;
    }
    force_sync(app).await
}
