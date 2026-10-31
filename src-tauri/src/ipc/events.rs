use crate::ipc::session::{app_calendar, client_for, store_state};
use crate::features::calendar::{AppCalendar, AppEvent, EventScope};
use crate::error::AppError;
use crate::features::calendar::ops;

/// # Errors
///
/// Returns an error when the account/store is unavailable or the `CalDAV` write
/// fails. The row is never kept locally on failure.
#[tauri::command]
pub async fn create_event(app: tauri::AppHandle, event: AppEvent) -> Result<(), AppError> {
    let client = client_for(&app).await?;
    let state = store_state(&app)?;
    let mut store = state.lock().await;
    ops::create_event(&mut store, &client, event).await?;
    Ok(())
}

/// # Errors
///
/// Returns an error when the event is missing or the `CalDAV` write fails.
#[tauri::command]
pub async fn update_event(app: tauri::AppHandle, event: AppEvent) -> Result<(), AppError> {
    let client = client_for(&app).await?;
    let state = store_state(&app)?;
    let mut store = state.lock().await;
    ops::update_event(&mut store, &client, event).await?;
    Ok(())
}

/// Delete one occurrence, the rest of the series, or the whole event.
///
/// `occurrence_start` is the canonical value of the clicked occurrence
/// (`EventInstance::occurrence_start`); it is required for every scope except
/// `all`.
///
/// # Errors
///
/// Returns an error when the `CalDAV` delete or rewrite fails.
#[tauri::command]
pub async fn delete_event(
    app: tauri::AppHandle,
    id: String,
    scope: EventScope,
    occurrence_start: Option<String>,
) -> Result<(), AppError> {
    let client = client_for(&app).await?;
    let state = store_state(&app)?;
    let mut store = state.lock().await;
    ops::delete_event(&mut store, &client, &id, scope, occurrence_start.as_deref()).await
}

/// Move the clicked occurrence or the whole series to a new time.
///
/// # Errors
///
/// Returns an error when the event is missing, the `CalDAV` write fails, or a
/// `following` move is requested (unsupported).
#[tauri::command]
pub async fn reschedule_event(
    app: tauri::AppHandle,
    id: String,
    start_time: String,
    end_time: String,
    is_all_day: bool,
    scope: EventScope,
    occurrence_start: Option<String>,
) -> Result<(), AppError> {
    let client = client_for(&app).await?;
    let state = store_state(&app)?;
    let mut store = state.lock().await;
    ops::reschedule_event(
        &mut store,
        &client,
        ops::Reschedule {
            id,
            start_time,
            end_time,
            is_all_day,
            scope,
            occurrence_start,
        },
    )
    .await
}

/// The store's discovered calendars, projected for the frontend.
///
/// # Errors
///
/// Returns an error if the store is not initialized.
#[tauri::command]
pub async fn get_active_calendars(
    app: tauri::AppHandle,
) -> Result<Vec<AppCalendar>, AppError> {
    let state = store_state(&app)?;
    Ok(state
        .read(|store| store.calendars.iter().map(app_calendar).collect())
        .await)
}
