use crate::error::AppError;
use crate::features::calendar::ops;
use crate::features::calendar::{AppCalendar, AppEvent, CalendarDateTime, EventScope};
use crate::ipc::session::{app_calendar, client_for, store_state};

/// # Errors
///
/// Returns an error when the account/store is unavailable or the `CalDAV` write
/// fails. The row is never kept locally on failure.
#[tauri::command]
#[specta::specta]
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
#[specta::specta]
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
#[specta::specta]
pub async fn delete_event(
    app: tauri::AppHandle,
    id: String,
    scope: EventScope,
    occurrence_start: Option<CalendarDateTime>,
) -> Result<(), AppError> {
    let client = client_for(&app).await?;
    let state = store_state(&app)?;
    let mut store = state.lock().await;
    ops::delete_event(&mut store, &client, &id, scope, occurrence_start).await
}

/// Move the clicked occurrence or the whole series to a new time.
///
/// # Errors
///
/// Returns an error when the event is missing, the `CalDAV` write fails, or a
/// `following` move is requested (unsupported).
#[tauri::command]
#[specta::specta]
pub async fn reschedule_event(
    app: tauri::AppHandle,
    id: String,
    start_time: CalendarDateTime,
    end_time: CalendarDateTime,
    is_all_day: bool,
    scope: EventScope,
    occurrence_start: Option<CalendarDateTime>,
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
#[specta::specta]
pub async fn get_active_calendars(
    app: tauri::AppHandle,
) -> Result<Vec<AppCalendar>, AppError> {
    let state = store_state(&app)?;
    Ok(state
        .read(|store| store.calendars.iter().map(app_calendar).collect())
        .await)
}
