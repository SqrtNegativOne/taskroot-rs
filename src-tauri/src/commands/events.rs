use crate::db;
use crate::domain;
use crate::error::AppError;

/// # Errors
///
/// Returns an error if the database is unavailable or the write fails.
#[tauri::command]
pub async fn create_event(
    app: tauri::AppHandle,
    mut event: domain::AppEvent,
) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;
    // New rows must carry the same denormalized calendar color the sync path
    // writes, or they render uncolored until their first round-trip.
    if event.color.is_none() {
        event.color = db::resolve_calendar_color(&pool, event.remote_collection_id.as_ref()).await?;
    }
    Ok(db::create_event(&pool, event).await?)
}

/// # Errors
///
/// Returns an error if the database is unavailable or the write fails.
#[tauri::command]
pub async fn update_event(app: tauri::AppHandle, event: domain::AppEvent) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;
    Ok(db::update_event(&pool, event).await?)
}

/// # Errors
///
/// Returns an error if the database is unavailable or the delete fails.
#[tauri::command]
pub async fn delete_event(app: tauri::AppHandle, id: String) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;
    Ok(db::delete_event(&pool, id).await?)
}

/// The stored calendars, active or not.
///
/// The `get_active_calendars` command body. It is split out of the handler so
/// the pool-level body is testable (see `src-tauri/AGENTS.md` → Command Testing).
///
/// # Errors
///
/// Returns an error if the query fails.
pub(crate) async fn active_calendars(
    pool: &sqlx::SqlitePool,
) -> Result<Vec<domain::AppCalendar>, AppError> {
    Ok(db::get_calendars(pool).await?)
}

/// # Errors
///
/// Returns an error if the database is unavailable.
#[tauri::command]
pub async fn get_active_calendars(
    app: tauri::AppHandle,
) -> Result<Vec<domain::AppCalendar>, AppError> {
    let pool = crate::db_pool(&app)?;
    active_calendars(&pool).await
}
