//! Shared `CalDAV` session setup for the app and the CLI.
//!
//! Both adapters resolve the data directory and decide what to load, then hand
//! the account and an HTTP client here; this is the one place that reads the
//! password and binds a [`CalDavClient`].

use std::path::Path;

use crate::error::AppError;

use super::account::{self, Account};
use super::client::CalDavClient;
use super::transport::HttpClient;

/// Environment variable consulted when the OS keychain has no password.
pub const PASSWORD_ENV: &str = "TASKROOT_CALDAV_PASSWORD";

/// Read the stored account, load its password and build a client.
///
/// The keychain is authoritative; [`PASSWORD_ENV`] is only a fallback for
/// headless runs.
///
/// # Errors
///
/// Returns [`AppError::Auth`] when no account or password is available and
/// [`AppError::Store`] when `account.json` is malformed.
pub fn connect(
    data_dir: &Path,
    http: &HttpClient,
    password_override: Option<&str>,
) -> Result<(Account, CalDavClient), AppError> {
    let account = account::read_account(data_dir)?
        .ok_or_else(|| AppError::Auth("No CalDAV account is configured".to_string()))?;
    let password = match password_override {
        Some(password) => password.to_string(),
        None => match account::load_password(&account.username) {
            Ok(Some(password)) => password,
            _ => std::env::var(PASSWORD_ENV).map_err(|_| {
                AppError::Auth(format!(
                    "The CalDAV password is missing from the OS keychain; set {PASSWORD_ENV}"
                ))
            })?,
        },
    };
    let client = CalDavClient::from_account(http, &account, &password)?;
    Ok((account, client))
}
