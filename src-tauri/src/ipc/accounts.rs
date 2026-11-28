//! Thin Tauri adapters for saved accounts. Lifecycle behavior lives in calendar.

use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use crate::error::AppError;
use crate::features::calendar::account::{Account, AccountRegistry, AccountService};
use crate::features::calendar::CalendarInfo;
use crate::ipc_events::{AccountChanged, StoreUpdated};

/// What the connect screen and the settings account card display.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummary {
    pub server_url: String,
    pub username: String,
    pub home_href: String,
    pub calendars: Vec<CalendarInfo>,
}

impl From<Account> for AccountSummary {
    fn from(account: Account) -> Self {
        Self {
            server_url: account.server_url,
            username: account.username,
            home_href: account.home_href,
            calendars: account.calendars,
        }
    }
}

pub(crate) fn service(app: &AppHandle) -> Result<State<'_, AccountService>, AppError> {
    app.try_state::<AccountService>()
        .ok_or_else(|| AppError::NotReady("The account service is not initialized".to_string()))
}

/// # Errors
/// Returns registry storage failures.
#[tauri::command]
#[specta::specta]
pub async fn is_account_configured(app: AppHandle) -> Result<bool, AppError> {
    Ok(service(&app)?.registry().await?.active().is_some())
}

/// # Errors
/// Returns registry storage failures.
#[tauri::command]
#[specta::specta]
pub async fn get_accounts(app: AppHandle) -> Result<AccountRegistry, AppError> {
    service(&app)?.registry().await
}

/// # Errors
/// Returns registry storage failures.
#[tauri::command]
#[specta::specta]
pub async fn get_account(app: AppHandle) -> Result<Option<Account>, AppError> {
    Ok(service(&app)?
        .registry()
        .await?
        .active()
        .map(|profile| profile.account.clone()))
}

/// # Errors
/// Returns discovery failures. Does not persist credentials or account data.
#[tauri::command]
#[specta::specta]
pub async fn test_account(
    app: AppHandle,
    server_url: String,
    username: String,
    password: String,
) -> Result<AccountSummary, AppError> {
    Ok(service(&app)?
        .test(&server_url, &username, &password)
        .await?
        .into())
}

/// # Errors
/// Returns discovery, fetch, credential or persistence failures.
#[tauri::command]
#[specta::specta]
pub async fn connect_account(
    app: AppHandle,
    server_url: String,
    username: String,
    password: String,
) -> Result<AccountSummary, AppError> {
    let state = super::session::store_state(&app)?;
    let account = service(&app)?
        .connect(&state, &server_url, &username, &password)
        .await?;
    changed(&app, true).await;
    Ok(account.into())
}

/// # Errors
/// Returns missing-profile, credential, fetch or persistence failures.
#[tauri::command]
#[specta::specta]
pub async fn switch_account(app: AppHandle, account_id: String) -> Result<(), AppError> {
    let state = super::session::store_state(&app)?;
    let accounts = service(&app)?;
    let previous = accounts.registry().await?.active_id;
    accounts.activate(&state, &account_id).await?;
    // Re-emit even when the id is unchanged: the fetch refreshes data, and the
    // frontend treats this event as "resources may have changed".
    changed(&app, previous.as_deref() != Some(&account_id)).await;
    Ok(())
}

/// Deactivate while retaining saved profiles for quick reconnection.
/// # Errors
/// Returns registry storage failures.
#[tauri::command]
#[specta::specta]
pub async fn disconnect_account(app: AppHandle) -> Result<(), AppError> {
    let state = super::session::store_state(&app)?;
    service(&app)?.disconnect(&state).await?;
    changed(&app, true).await;
    Ok(())
}

/// # Errors
/// Returns missing-profile, credential or persistence failures.
#[tauri::command]
#[specta::specta]
pub async fn remove_account(app: AppHandle, account_id: String) -> Result<(), AppError> {
    let accounts = service(&app)?;
    let was_active = accounts.registry().await?.active_id.as_deref() == Some(&account_id);
    let state = super::session::store_state(&app)?;
    accounts.remove(&state, &account_id).await?;
    changed(&app, was_active).await;
    Ok(())
}

async fn changed(app: &AppHandle, clear_focus: bool) {
    if clear_focus {
        super::stopwatch::clear_account_focus(app).await;
        let _ = AccountChanged.emit(app);
    }
    let _ = StoreUpdated.emit(app);
}
