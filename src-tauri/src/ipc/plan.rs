//! Plan-screen queries: tasks, raw events and render-ready event instances.

use crate::ipc::session::store_state;
use crate::features::calendar::{expand_event_instances, single_instance};
use crate::features::calendar::{AppEvent, AppEventFilter, AppTask, AppTaskFilter, AppTaskSort};
use crate::error::AppError;
use crate::features::calendar::query;

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
) -> Result<Vec<crate::features::calendar::EventInstance>, AppError> {
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
pub async fn get_event(app: tauri::AppHandle, id: String) -> Result<Option<AppEvent>, AppError> {
    let state = store_state(&app)?;
    Ok(state.read(|store| store.event(&id).cloned()).await)
}
