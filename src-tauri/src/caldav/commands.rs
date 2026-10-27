//! Tauri commands for `CalDAV` account setup.
//!
//! Discovery is the only network work here: `test_account` runs it without
//! persisting anything and `connect_account` persists the account. The collection
//! list is kept fresh at startup by `discovery::refresh_collections`, so there is
//! no separate refresh command. Calendar data does not flow through these
//! commands.

use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use crate::error::AppError;

use super::account::{self, Account};
use super::discovery::{self, Discovery};
use super::model::{CalendarComponent, CalendarInfo};
use super::transport::HttpClient;

/// What the connect screen and the settings account card display.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, TS)]
#[ts(
    export,
    export_to = "../../src/lib/bindings/AccountSummary.generated.ts"
)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummary {
    pub server_url: String,
    pub username: String,
    pub home_href: String,
    pub calendars: Vec<CalendarInfo>,
}

impl AccountSummary {
    fn from_account(account: &Account) -> Self {
        Self {
            server_url: account.server_url.clone(),
            username: account.username.clone(),
            home_href: account.home_href.clone(),
            calendars: account.calendars.clone(),
        }
    }

    fn from_discovery(discovery: &Discovery, username: &str) -> Self {
        Self {
            server_url: discovery.server_url.clone(),
            username: username.to_string(),
            home_href: discovery.home_href.clone(),
            calendars: discovery.calendars.clone(),
        }
    }
}

/// Whether an account is stored on disk.
///
/// # Errors
///
/// Returns [`AppError::Store`] when `account.json` exists but is unreadable.
#[tauri::command]
pub async fn is_account_configured(app: AppHandle) -> Result<bool, AppError> {
    let dir = crate::app_data_dir(&app)?;
    Ok(account::read_account(&dir)?.is_some())
}

/// The stored account, or `None` when the app is not connected.
///
/// # Errors
///
/// Returns [`AppError::Store`] when `account.json` is malformed.
#[tauri::command]
pub async fn get_account(app: AppHandle) -> Result<Option<Account>, AppError> {
    let dir = crate::app_data_dir(&app)?;
    account::read_account(&dir)
}

/// Run discovery and return the collections it found, without persisting.
///
/// # Errors
///
/// Returns [`AppError::InvalidInput`] for a bad URL, [`AppError::Auth`] for bad
/// credentials and [`AppError::Sync`] when the server does not speak `CalDAV`.
#[tauri::command]
pub async fn test_account(
    app: AppHandle,
    server_url: String,
    username: String,
    password: String,
) -> Result<AccountSummary, AppError> {
    let http = http_client(&app)?;
    let discovery = discovery::discover(&http, &server_url, &username, &password).await?;
    Ok(AccountSummary::from_discovery(&discovery, &username))
}

/// Discover, then persist `account.json` and the keychain password.
///
/// # Errors
///
/// Propagates discovery errors and returns [`AppError::Store`] when either the
/// file or the keychain write fails.
#[tauri::command]
pub async fn connect_account(
    app: AppHandle,
    server_url: String,
    username: String,
    password: String,
) -> Result<AccountSummary, AppError> {
    let dir = crate::app_data_dir(&app)?;
    let http = http_client(&app)?;
    let discovery = discovery::discover(&http, &server_url, &username, &password).await?;

    let Discovery {
        server_url,
        principal_href,
        home_href,
        calendars,
        ..
    } = discovery;
    let account = Account {
        server_url,
        username: username.clone(),
        principal_href,
        home_href,
        calendars,
    };

    account::save_password(&username, &password)?;
    if let Err(error) = account::write_account(&dir, &account) {
        if let Err(cleanup_error) = account::delete_password(&username) {
            eprintln!("[caldav] could not remove the keychain password after a failed connect: {cleanup_error}");
        }
        return Err(error);
    }

    Ok(AccountSummary::from_account(&account))
}

/// Delete `account.json` and the keychain password.
///
/// # Errors
///
/// Returns [`AppError::Store`] when either deletion fails. The account file is
/// deleted first, so a failed keychain delete still leaves a retryable state
/// (the account remains until the file is gone).
#[tauri::command]
pub async fn disconnect_account(app: AppHandle) -> Result<(), AppError> {
    let dir = crate::app_data_dir(&app)?;
    let account = account::read_account(&dir)?;
    let password_result = account.as_ref().map_or(Ok(()), |account| {
        account::delete_password(&account.username)
    });
    let file_result = account::delete_account_file(&dir);

    password_result?;
    file_result
}

/// Create a list and refresh the mirrored collection list.
///
/// # Errors
///
/// Returns the account/`MKCALENDAR`/re-discovery failures.
#[tauri::command]
pub async fn create_calendar(
    app: AppHandle,
    display_name: String,
    component: CalendarComponent,
) -> Result<(), AppError> {
    let dir = crate::app_data_dir(&app)?;
    let client = crate::commands::session::client_for(&app).await?;
    let state = crate::commands::session::store_state(&app)?;
    let mut store = state.lock().await;
    super::calendars::create(&dir, &client, &mut store, &display_name, component).await
}

/// Rename a list.
///
/// # Errors
///
/// Returns [`AppError::NotFound`]/[`AppError::InvalidInput`] for an unknown or
/// read-only list and propagates the `PROPPATCH` failure.
#[tauri::command]
pub async fn rename_calendar(
    app: AppHandle,
    href: String,
    display_name: String,
) -> Result<(), AppError> {
    let dir = crate::app_data_dir(&app)?;
    let client = crate::commands::session::client_for(&app).await?;
    let state = crate::commands::session::store_state(&app)?;
    let mut store = state.lock().await;
    super::calendars::rename(&dir, &client, &mut store, &href, &display_name).await
}

/// Set or clear (`color: null`) a list's colour.
///
/// # Errors
///
/// Returns [`AppError::InvalidInput`] for a malformed colour or read-only list
/// and propagates the `PROPPATCH` failure.
#[tauri::command]
pub async fn set_calendar_color(
    app: AppHandle,
    href: String,
    color: Option<String>,
) -> Result<(), AppError> {
    let dir = crate::app_data_dir(&app)?;
    let client = crate::commands::session::client_for(&app).await?;
    let state = crate::commands::session::store_state(&app)?;
    let mut store = state.lock().await;
    super::calendars::set_color(&dir, &client, &mut store, &href, color).await
}

/// Delete a list, its resources and its sync token.
///
/// # Errors
///
/// Returns [`AppError::NotFound`]/[`AppError::InvalidInput`] for an unknown or
/// read-only list and propagates the `DELETE` failure.
#[tauri::command]
pub async fn delete_calendar(app: AppHandle, href: String) -> Result<(), AppError> {
    let dir = crate::app_data_dir(&app)?;
    let client = crate::commands::session::client_for(&app).await?;
    let state = crate::commands::session::store_state(&app)?;
    let mut store = state.lock().await;
    super::calendars::delete(&dir, &client, &mut store, &href).await
}

fn http_client(app: &AppHandle) -> Result<HttpClient, AppError> {
    app.try_state::<HttpClient>()
        .map(|state| HttpClient(state.0.clone()))
        .ok_or_else(|| AppError::NotReady("The HTTP client is not initialized".to_string()))
}
