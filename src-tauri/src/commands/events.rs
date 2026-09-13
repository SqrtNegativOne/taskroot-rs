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
    // New rows must carry the same denormalized calendar color the sync path
    // writes, or they render uncolored until their first round-trip.
    if event.color.is_none() {
        event.color = db::resolve_calendar_color(&pool, event.remote_collection_id.as_ref()).await?;
    }
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
    let previous = db::get_event(&pool, &event.id.0).await?;

    match sync::push::plan_event_sync(previous.as_ref(), &event) {
        sync::push::EventSyncPlan::Move { source_calendar_id } => {
            sync::push::push_event_move_or_enqueue(&app, &mut event, source_calendar_id).await;
        }
        sync::push::EventSyncPlan::Update => {
            sync::push::push_or_enqueue(&app, &mut event, sync::types::SyncAction::Update).await;
        }
    }

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
