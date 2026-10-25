mod expand;

pub use expand::{expand_event_instances, single_instance};

use crate::commands::session::{client_for, store_state};
use crate::domain::{AppEvent, AppEventFilter, AppTask, AppTaskFilter, AppTaskSort};
use crate::error::AppError;
use crate::store::{ops, query};

/// # Errors
///
/// Returns an error if the store is not initialized.
#[tauri::command]
pub async fn query_tasks(
    app: tauri::AppHandle,
    filters: Vec<AppTaskFilter>,
    sort: Vec<AppTaskSort>,
    query: String,
) -> Result<Vec<AppTask>, AppError> {
    let state = store_state(&app)?;
    Ok(state
        .read(|store| query::query_tasks(&store.tasks(), &filters, &sort, &query))
        .await)
}

/// Return raw events for the inspector (wire-shaped for the write path).
///
/// # Errors
///
/// Returns an error if the store is not initialized.
#[tauri::command]
pub async fn query_events(
    app: tauri::AppHandle,
    filters: Vec<AppEventFilter>,
    query: String,
) -> Result<Vec<AppEvent>, AppError> {
    let state = store_state(&app)?;
    Ok(state
        .read(|store| query::query_events(&store.events(), &filters, &query))
        .await)
}

/// Return render-ready instances so callers never parse wire timestamps.
///
/// # Errors
///
/// Returns an error if the store is not initialized.
#[tauri::command]
pub async fn query_event_instances(
    app: tauri::AppHandle,
    filters: Vec<AppEventFilter>,
    query: String,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<Vec<crate::domain::EventInstance>, AppError> {
    let state = store_state(&app)?;
    let events = state
        .read(|store| query::query_events(&store.events(), &filters, &query))
        .await;

    match (start_date, end_date) {
        (Some(start), Some(end)) => Ok(expand_event_instances(&events, &start, &end)),
        _ => Ok(events.iter().filter_map(single_instance).collect()),
    }
}

/// Fetch one raw event by id (used to reschedule instances back onto the master).
///
/// # Errors
///
/// Returns an error if the store is not initialized.
#[tauri::command]
pub async fn get_event(
    app: tauri::AppHandle,
    id: String,
) -> Result<Option<AppEvent>, AppError> {
    let state = store_state(&app)?;
    Ok(state.read(|store| store.event(&id).cloned()).await)
}

/// Update a master event's start/end timestamps after a drag or resize.
///
/// Preserves the event's timezone so the write keeps `DTSTART;TZID=…` and the
/// recurrence stays anchored to the local wall clock.
///
/// # Errors
///
/// Returns an error if the event is missing or the `CalDAV` write fails.
#[tauri::command]
pub async fn reschedule_event(
    app: tauri::AppHandle,
    id: String,
    start_time: String,
    end_time: String,
    is_all_day: bool,
) -> Result<(), AppError> {
    let state = store_state(&app)?;
    let mut event = {
        let store = state.lock().await;
        store
            .event(&id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("event {id}")))?
    };
    event.start_time = start_time;
    event.end_time = end_time;
    event.is_all_day = Some(is_all_day);
    let client = client_for(&app).await?;
    let mut store = state.lock().await;
    ops::update_event(&mut store, &client, event).await
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
