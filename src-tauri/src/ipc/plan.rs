//! Plan-screen queries: tasks, raw events and render-ready event instances.

use tauri_specta::Event;

use crate::error::AppError;
use crate::features::calendar::{ops, query};
use crate::features::calendar::{
    expand_task_dates, query_event_instances as project_event_instances,
};
use crate::features::calendar::{
    AppEvent, AppEventFilter, AppEventSort, AppTask, AppTaskFilter, AppTaskSort, FilterGroup,
    TaskDateInstance,
};
use crate::ipc::session::{client_for, store_state};
use crate::ipc_events::StoreUpdated;

/// # Errors
///
/// Returns an error if the store is not initialized.
#[tauri::command]
#[specta::specta]
pub async fn query_tasks(
    app: tauri::AppHandle,
    filters: FilterGroup<AppTaskFilter>,
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
#[specta::specta]
pub async fn query_events(
    app: tauri::AppHandle,
    filters: FilterGroup<AppEventFilter>,
    sort: Vec<AppEventSort>,
    query: String,
) -> Result<Vec<AppEvent>, AppError> {
    let state = store_state(&app)?;
    Ok(state
        .read(|store| query::query_events(&store.events(), &filters, &sort, &query))
        .await)
}

/// Return render-ready instances so callers never parse wire timestamps.
///
/// # Errors
///
/// Returns an error if the store is not initialized.
#[tauri::command]
#[specta::specta]
pub async fn query_event_instances(
    app: tauri::AppHandle,
    filters: FilterGroup<AppEventFilter>,
    sort: Vec<AppEventSort>,
    query: String,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<Vec<crate::features::calendar::EventInstance>, AppError> {
    let state = store_state(&app)?;
    let instances = state
        .read(|store| {
            let range = start_date.as_deref().zip(end_date.as_deref());
            let mut insts =
                project_event_instances(&store.events(), &filters, &sort, &query, range);
            for inst in &mut insts {
                if let Some(ref tid) = inst.task_id {
                    if let Some(task) = store.task(tid.as_str()) {
                        inst.task_status = task.status.clone();
                    }
                }
            }
            insts
        })
        .await;

    Ok(instances)
}

/// Return every distinct tag name for the filter pickers.
///
/// # Errors
///
/// Returns an error if the store is not initialized.
#[tauri::command]
#[specta::specta]
pub async fn list_tags(app: tauri::AppHandle) -> Result<Vec<String>, AppError> {
    let state = store_state(&app)?;
    Ok(state
        .read(crate::features::calendar::Store::tag_names)
        .await)
}

/// Rename one tag across every task and event in the store.
///
/// # Errors
///
/// Returns an error if the new name is invalid, any affected calendar is
/// read-only, the account/store is unavailable, or a `CalDAV` write fails.
#[tauri::command]
#[specta::specta]
pub async fn rename_tag(
    app: tauri::AppHandle,
    from: String,
    to: String,
) -> Result<(), AppError> {
    let client = client_for(&app).await?;
    let state = store_state(&app)?;
    ops::rename_tag(&state, &client, &from, &to).await?;
    let _ = StoreUpdated.emit(&app);
    Ok(())
}

/// Return task start/due markers inside a local date window.
///
/// # Errors
///
/// Returns an error if the store is not initialized.
#[tauri::command]
#[specta::specta]
pub async fn query_task_dates(
    app: tauri::AppHandle,
    start_date: String,
    end_date: String,
) -> Result<Vec<TaskDateInstance>, AppError> {
    let state = store_state(&app)?;
    let tasks = state.read(crate::features::calendar::Store::tasks).await;
    Ok(expand_task_dates(&tasks, &start_date, &end_date))
}
