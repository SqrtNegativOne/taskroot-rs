use crate::db;
use crate::domain;
use crate::error::AppError;
use crate::sync;



/// # Errors
///
/// Returns an error if the database is unavailable or the write fails.
#[tauri::command]
pub async fn create_event(
    app: tauri::AppHandle,
    mut event: domain::AppEvent,
) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;
    sync::push::push_or_enqueue(&app, &mut event, sync::types::SyncAction::Create).await;
    Ok(db::create_event(&pool, event).await?)
}

/// # Errors
///
/// Returns an error if the database is unavailable or the write fails.
#[tauri::command]
pub async fn update_event(
    app: tauri::AppHandle,
    mut event: domain::AppEvent,
) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;
    sync::push::push_or_enqueue(&app, &mut event, sync::types::SyncAction::Update).await;
    Ok(db::update_event(&pool, event).await?)
}

/// # Errors
///
/// Returns an error if the database is unavailable or the delete fails.
#[tauri::command]
pub async fn delete_event(app: tauri::AppHandle, id: String) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;

    if let Ok(Some(event)) = db::get_event(&pool, &id).await {
        let app_clone = app.clone();
        tokio::spawn(async move {
            sync::push::push_delete_or_enqueue(&app_clone, &event).await;
        });
    }

    Ok(db::delete_event(&pool, id).await?)
}

/// # Errors
///
/// Returns an error if the database is unavailable.
#[tauri::command]
pub async fn get_active_calendars(app: tauri::AppHandle) -> Result<Vec<domain::AppCalendar>, AppError> {
    let pool = crate::db_pool(&app)?;
    Ok(db::get_calendars(&pool).await?)
}
