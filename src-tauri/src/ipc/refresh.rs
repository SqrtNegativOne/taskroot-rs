//! Manual refresh command.

use tauri::AppHandle;

use crate::error::AppError;

/// Reload every collection from the `CalDAV` server.
///
/// # Errors
///
/// Returns the account/transport/parse failure from the load.
#[tauri::command]
pub async fn refresh_data(app: AppHandle) -> Result<(), AppError> {
    super::session::load_all(&app).await
}
