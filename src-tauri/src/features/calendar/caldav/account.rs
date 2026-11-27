//! Saved `CalDAV` profiles and the active-account registry (no passwords).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use super::model::CalendarInfo;
use crate::error::AppError;

mod credentials;
mod leases;
mod management;
mod service;
mod storage;

pub use credentials::{CredentialStore, KeychainCredentials};
pub use leases::{AccountScope, ClientLease};
pub use service::AccountService;
pub use storage::{AccountRepository, JsonAccountRepository};

pub const ACCOUNT_FILE: &str = "account.json";

/// The persisted account. Contains no credentials.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub server_url: String,
    pub username: String,
    pub principal_href: String,
    pub home_href: String,
    pub calendars: Vec<CalendarInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AccountProfile {
    pub id: String,
    pub account: Account,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountRegistry {
    pub active_id: Option<String>,
    pub profiles: Vec<AccountProfile>,
}

impl AccountRegistry {
    #[must_use]
    pub fn active(&self) -> Option<&AccountProfile> {
        self.active_id
            .as_ref()
            .and_then(|id| self.profiles.iter().find(|profile| &profile.id == id))
    }

    pub(super) fn profile(&self, id: &str) -> Result<&AccountProfile, AppError> {
        self.profiles
            .iter()
            .find(|profile| profile.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Account {id}")))
    }
}

#[must_use]
pub fn account_path(dir: &Path) -> PathBuf {
    dir.join(ACCOUNT_FILE)
}

/// Read the active account; malformed registries are errors, not silent resets.
///
/// # Errors
/// Returns a storage error when the registry cannot be read or validated.
pub fn read_account(dir: &Path) -> Result<Option<Account>, AppError> {
    Ok(JsonAccountRepository::new(dir.to_path_buf())
        .read()?
        .active()
        .map(|profile| profile.account.clone()))
}

/// Read metadata for the client's pinned server/user, even after disk activation changes.
/// # Errors
/// Returns registry storage failures.
pub fn read_client_account(
    dir: &Path,
    client: &super::client::CalDavClient,
) -> Result<Option<Account>, AppError> {
    let registry = JsonAccountRepository::new(dir.to_path_buf()).read()?;
    Ok(registry
        .profiles
        .into_iter()
        .find(|profile| {
            profile.account.server_url.trim_end_matches('/')
                == client.base_url().as_str().trim_end_matches('/')
                && profile.account.username == client.username()
        })
        .map(|profile| profile.account))
}

/// Update discovery metadata without changing the active profile.
/// An empty registry is initialized for headless fixture setup.
///
/// # Errors
/// Returns a storage error or rejects an account no longer in the registry.
pub fn write_account(dir: &Path, account: &Account) -> Result<(), AppError> {
    let repository = JsonAccountRepository::new(dir.to_path_buf());
    let mut registry = repository.read()?;
    if let Some(profile) = registry.profiles.iter_mut().find(|profile| {
        profile.account.server_url == account.server_url
            && profile.account.username == account.username
    }) {
        profile.account = account.clone();
    } else if registry.profiles.is_empty() {
        let id = uuid::Uuid::new_v4().to_string();
        registry.active_id = Some(id.clone());
        registry.profiles.push(AccountProfile {
            id,
            account: account.clone(),
        });
    } else {
        return Err(AppError::NotFound("The account was removed".to_string()));
    }
    repository.write(&registry)
}

/// Remove a disposable registry file. Does not touch credentials.
///
/// # Errors
/// Returns a storage error on filesystem failure.
pub fn delete_account_file(dir: &Path) -> Result<(), AppError> {
    match std::fs::remove_file(account_path(dir)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::Store(format!(
            "Could not remove account file: {error}"
        ))),
    }
}
