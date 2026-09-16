mod expand;

pub use expand::{expand_event_instances, single_instance};

use crate::db;
use crate::domain::{AppEvent, AppEventFilter, AppTask, AppTaskFilter};
use crate::error::AppError;
use crate::sync;

/// # Errors
///
/// Returns an error if the operation fails.
#[tauri::command]
pub async fn query_tasks(
    app: tauri::AppHandle,
    filters: Vec<AppTaskFilter>,
    sort: Vec<crate::domain::AppTaskSort>,
    query: String,
) -> Result<Vec<AppTask>, AppError> {
    let pool = crate::db_pool(&app)?;
    Ok(db::query_tasks(&pool, filters, sort, query).await?)
}

/// Return raw mirrored events (wire format preserved for the push path).
///
/// # Errors
///
/// Returns an error if the operation fails.
#[tauri::command]
pub async fn query_events(
    app: tauri::AppHandle,
    filters: Vec<AppEventFilter>,
    query: String,
) -> Result<Vec<AppEvent>, AppError> {
    let pool = crate::db_pool(&app)?;
    Ok(db::query_events(&pool, filters, query).await?)
}

/// Return render-ready instances so callers never parse wire timestamps.
///
/// # Errors
///
/// Returns an error if the operation fails.
#[tauri::command]
pub async fn query_event_instances(
    app: tauri::AppHandle,
    filters: Vec<AppEventFilter>,
    query: String,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<Vec<crate::domain::EventInstance>, AppError> {
    let pool = crate::db_pool(&app)?;
    let events = db::query_events(&pool, filters, query).await?;

    match (start_date, end_date) {
        (Some(start), Some(end)) => Ok(expand_event_instances(&events, &start, &end)),
        _ => Ok(events.iter().filter_map(single_instance).collect()),
    }
}

/// Fetch one raw event by id (used to reschedule instances back onto the master).
///
/// # Errors
///
/// Returns an error if the operation fails.
#[tauri::command]
pub async fn get_event(
    app: tauri::AppHandle,
    id: String,
) -> Result<Option<AppEvent>, AppError> {
    let pool = crate::db_pool(&app)?;
    Ok(db::get_event(&pool, &id).await?)
}

/// Update a master event's start/end timestamps after a drag or resize.
///
/// When `is_all_day` is true, `start_time`/`end_time` are floating
/// `YYYY-MM-DD` dates so a grid-to-grid drag keeps an all-day event all-day.
///
/// # Errors
///
/// Returns an error if the event is missing or the update fails.
#[tauri::command]
pub async fn reschedule_event(
    app: tauri::AppHandle,
    id: String,
    start_time: String,
    end_time: String,
    is_all_day: bool,
) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;
    let Some(mut event) = db::get_event(&pool, &id).await? else {
        return Err(AppError::NotFound(format!("event {id}")));
    };
    event.start_time = start_time;
    event.end_time = end_time;
    event.is_all_day = Some(is_all_day);
    // A drag/resize must reach Google, not just SQLite. `push_or_enqueue`
    // mirrors `commands::events::update_event`: it marks the row dirty and
    // queues an Update (or triggers an immediate sync).
    //
    // Note: this moves the row it was given. For a rule-expanded occurrence the
    // frontend passes the master's id, so the whole series shifts; per-occurrence
    // moves are part of the exception-instance follow-up (see
    // `docs/google-calendar-write.md`).
    sync::push::push_or_enqueue(&app, &mut event, sync::types::SyncAction::Update).await;
    db::update_event(&pool, event).await?;
    Ok(())
}

#[tauri::command]
#[must_use]
pub fn get_task_schema() -> Vec<crate::domain::AppTaskColumnDef> {
    crate::domain::AppTask::get_schema()
}

#[tauri::command]
#[must_use]
pub fn get_event_schema() -> Vec<crate::domain::AppEventColumnDef> {
    crate::domain::AppEvent::get_schema()
}
