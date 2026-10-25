//! Shared plumbing for commands that touch the `CalDAV` account or the store.

use tauri::{AppHandle, Manager, State};

use crate::caldav::account;
use crate::caldav::client::CalDavClient;
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
    let account = account::read_account(&dir)?
        .ok_or_else(|| AppError::Auth("No CalDAV account is configured".to_string()))?;
    let password = match account::load_password(&account.username) {
        Ok(Some(password)) => password,
        _ => std::env::var("TASKROOT_CalDAV_PASSWORD").map_err(|_| {
            AppError::Auth("The CalDAV password is missing from the OS keychain".to_string())
        })?,
    };
    let http = app
        .try_state::<HttpClient>()
        .ok_or_else(|| AppError::Internal("The HTTP client is not initialized".to_string()))?;
    CalDavClient::from_account(&http, &account, &password)
}

/// # Errors
///
/// Returns [`AppError::Internal`] when the store was never initialized.
pub(crate) fn store_state(app: &AppHandle) -> Result<State<'_, StoreState>, AppError> {
    app.try_state::<StoreState>()
        .ok_or_else(|| AppError::Internal("The store is not initialized".to_string()))
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
