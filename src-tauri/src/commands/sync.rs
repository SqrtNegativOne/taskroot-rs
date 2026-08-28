use crate::error::AppError;
use crate::sync;


/// # Errors
///
/// Returns [`AppError::NotReady`] when the database is not initialized and
/// [`AppError::Sync`] when the Google sync round fails.
#[tauri::command]
pub async fn force_sync(app: tauri::AppHandle) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;
    let next_sync = chrono::Utc::now().checked_add_signed(chrono::Duration::seconds(sync::SYNC_INTERVAL_SECS)).unwrap_or_else(chrono::Utc::now);
    sync::run_tracked_sync(&app, &pool, Some(next_sync))
        .await
        .map_err(AppError::Sync)
}

/// # Errors
/// Returns an error if the database operation fails.
#[tauri::command]
pub async fn wipe_local_data(app: tauri::AppHandle) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;
    sqlx::query("DELETE FROM tasks").execute(&*pool).await?;
    sqlx::query("DELETE FROM events").execute(&*pool).await?;
    sqlx::query("DELETE FROM sync_queue")
        .execute(&*pool)
        .await?;
    Ok(())
}

/// # Errors
/// Returns an error if the database operation fails.
#[tauri::command]
pub async fn clear_sync_queue(app: tauri::AppHandle) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;
    sqlx::query("DELETE FROM sync_queue")
        .execute(&*pool)
        .await?;
    Ok(())
}

/// # Errors
/// Returns an error if the database operation fails.
#[tauri::command]
pub async fn get_sync_queue(app: tauri::AppHandle) -> Result<Vec<serde_json::Value>, AppError> {
    let pool = crate::db_pool(&app)?;
    let rows = sqlx::query("SELECT payload FROM sync_queue ORDER BY id ASC")
        .fetch_all(&*pool)
        .await?;

    let mut items = Vec::new();
    for row in rows {
        let payload: String = sqlx::Row::try_get(&row, "payload").map_err(AppError::Db)?;
        if let Ok(val) = serde_json::from_str(&payload) {
            items.push(val);
        }
    }
    Ok(items)
}
