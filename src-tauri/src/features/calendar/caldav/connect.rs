//! Headless session setup; dependencies can be replaced without OS keychain access.

use std::path::Path;

use super::account::{
    Account, AccountRepository, CredentialStore, JsonAccountRepository, KeychainCredentials,
};
use super::client::CalDavClient;
use super::transport::HttpClient;
use crate::error::AppError;

pub const PASSWORD_ENV: &str = "TASKROOT_CALDAV_PASSWORD";

/// Resolve the active profile for a headless run. An explicit environment
/// password overrides the keychain for this process only.
/// # Errors
/// Returns account, credential or URL failures.
pub fn connect(
    data_dir: &Path,
    http: &HttpClient,
    password_override: Option<&str>,
) -> Result<(Account, CalDavClient), AppError> {
    let environment = std::env::var(PASSWORD_ENV).ok();
    connect_with(
        &JsonAccountRepository::new(data_dir.to_path_buf()),
        &KeychainCredentials,
        http,
        password_override.or(environment.as_deref()),
    )
}

/// # Errors
/// Returns account, credential or URL failures.
pub fn connect_with(
    repository: &dyn AccountRepository,
    credentials: &dyn CredentialStore,
    http: &HttpClient,
    password_override: Option<&str>,
) -> Result<(Account, CalDavClient), AppError> {
    let registry = repository.read()?;
    let profile = registry
        .active()
        .ok_or_else(|| AppError::Auth("No CalDAV account is active".to_string()))?;
    let password = match password_override {
        Some(password) => password.to_string(),
        None => credentials.load(&profile.id)?.ok_or_else(|| {
            AppError::Auth(format!(
                "The saved password is missing; reconnect or set {PASSWORD_ENV}"
            ))
        })?,
    };
    let client = CalDavClient::from_account(http, &profile.account, &password)?;
    Ok((profile.account.clone(), client))
}
