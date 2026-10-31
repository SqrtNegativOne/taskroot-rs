//! Shared plumbing for commands that touch the `CalDAV` account or the store.

use tauri::{AppHandle, Manager, State};

use crate::caldav::client::CalDavClient;
use crate::caldav::connect;
use crate::caldav::model::CalendarInfo;
use crate::caldav::transport::HttpClient;
use crate::domain::{AppCalendar, CollectionId};
use crate::error::AppError;
use crate::store::StoreState;

/// The process-wide HTTP client plus the stored account credentials.
///
/// # Errors
///
/// Returns [`AppError::Auth`] when no account/password is available and
/// [`AppError::Internal`] when the HTTP client or store was never initialized.
pub(crate) async fn client_for(app: &AppHandle) -> Result<CalDavClient, AppError> {
    let dir = crate::app_data_dir(app)?;
    let http = app
        .try_state::<HttpClient>()
        .ok_or_else(|| AppError::NotReady("The HTTP client is not initialized".to_string()))?
        .inner()
        .clone();
    let (_account, client) = connect::connect(&dir, &http, None)?;
    Ok(client)
}

/// # Errors
///
/// Returns [`AppError::Internal`] when the store was never initialized.
pub(crate) fn store_state(app: &AppHandle) -> Result<State<'_, StoreState>, AppError> {
    app.try_state::<StoreState>()
        .ok_or_else(|| AppError::NotReady("The store is not initialized".to_string()))
}

/// Project a discovered collection onto the view DTO the frontend consumes.
#[must_use]
pub(crate) fn app_calendar(info: &CalendarInfo) -> AppCalendar {
    AppCalendar {
        id: CollectionId::from(info.href.clone()),
        summary: info.display_name.clone(),
        color: info
            .color
            .as_deref()
            .and_then(|color| crate::domain::Color::try_from(color.to_string()).ok()),
        writable: info.writable,
        component: info.component,
    }
}
