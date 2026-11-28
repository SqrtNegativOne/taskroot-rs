//! Saved `CalDAV` profiles and the active-account registry (no passwords).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use super::model::CalendarInfo;
use crate::error::AppError;

mod calendars;
mod credentials;
mod leases;
mod management;
mod service;
mod storage;

pub use credentials::{CredentialStore, KeychainCredentials, PASSWORD_ENV};
pub use leases::{AccountScope, ClientLease};
pub use service::AccountService;
pub use storage::{AccountRepository, JsonAccountRepository};

pub use crate::storage::ACCOUNT_FILE;

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
