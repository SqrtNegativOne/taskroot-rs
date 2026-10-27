//! The single `CalDAV` account: `account.json` plus the OS keychain password.
//!
//! `account.json` never contains the password. It is written atomically (a
//! sibling `.tmp` file is renamed over it) and a malformed file is a
//! [`AppError::Store`] failure rather than a silent reset, so a broken account
//! cannot be mistaken for "no account configured".
//!
//! The password lives in the OS keychain under [`KEYRING_SERVICE`], keyed by
//! the username.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;

use super::model::CalendarInfo;

/// File name of the account record inside the app data directory.
pub const ACCOUNT_FILE: &str = "account.json";
/// Keychain service name for the `CalDAV` password.
pub const KEYRING_SERVICE: &str = "taskroot.caldav";

const TEMP_FILE: &str = "account.json.tmp";

/// The persisted account. Contains no credentials.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/Account.generated.ts")]
#[serde(rename_all = "camelCase", default)]
pub struct Account {
    pub server_url: String,
    pub username: String,
    pub principal_href: String,
    pub home_href: String,
    pub calendars: Vec<CalendarInfo>,
}

/// `<dir>/account.json`.
#[must_use]
pub fn account_path(dir: &Path) -> PathBuf {
    dir.join(ACCOUNT_FILE)
}

/// Read the account, or `None` when no account has been configured.
///
/// # Errors
///
/// Returns [`AppError::Store`] when the file exists but cannot be read or
/// parsed.
pub fn read_account(dir: &Path) -> Result<Option<Account>, AppError> {
    let path = account_path(dir);
    match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).map(Some).map_err(|error| {
            AppError::Store(format!(
                "Malformed account file `{}`: {error}",
                path.display()
            ))
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(AppError::Store(format!(
            "Could not read account file `{}`: {error}",
            path.display()
        ))),
    }
}

/// Write the account atomically: a `.tmp` file is renamed over `account.json`.
///
/// # Errors
///
/// Returns [`AppError::Store`] when the file cannot be serialised or written.
pub fn write_account(dir: &Path, account: &Account) -> Result<(), AppError> {
    let path = account_path(dir);
    let temp_path = dir.join(TEMP_FILE);
    let json = serde_json::to_vec_pretty(account)
        .map_err(|error| AppError::Store(format!("Could not encode the account: {error}")))?;

    std::fs::write(&temp_path, json).map_err(|error| {
        AppError::Store(format!(
            "Could not write account file `{}`: {error}",
            temp_path.display()
        ))
    })?;

    std::fs::rename(&temp_path, &path).map_err(|error| {
        if let Err(cleanup_error) = std::fs::remove_file(&temp_path) {
            eprintln!(
                "[caldav] could not remove `{}` after a failed rename: {cleanup_error}",
                temp_path.display()
            );
        }
        AppError::Store(format!(
            "Could not replace account file `{}`: {error}",
            path.display()
        ))
    })
}

/// Delete `account.json`. A missing file counts as success.
///
/// # Errors
///
/// Returns [`AppError::Store`] when the file exists but cannot be removed.
pub fn delete_account_file(dir: &Path) -> Result<(), AppError> {
    let path = account_path(dir);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::Store(format!(
            "Could not remove account file `{}`: {error}",
            path.display()
        ))),
    }
}

/// Store the password in the OS keychain.
///
/// # Errors
///
/// Returns [`AppError::Store`] when the keychain rejects the write.
pub fn save_password(username: &str, password: &str) -> Result<(), AppError> {
    let entry =
        keyring::Entry::new(KEYRING_SERVICE, username).map_err(|error| keychain_error(&error))?;
    entry
        .set_password(password)
        .map_err(|error| keychain_error(&error))
}

/// Load the password from the OS keychain, or `None` when it is absent.
///
/// # Errors
///
/// Returns [`AppError::Store`] when the keychain cannot be queried.
pub fn load_password(username: &str) -> Result<Option<String>, AppError> {
    let entry =
        keyring::Entry::new(KEYRING_SERVICE, username).map_err(|error| keychain_error(&error))?;
    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(keychain_error(&error)),
    }
}

/// Delete the keychain password. A missing entry counts as success.
///
/// # Errors
///
/// Returns [`AppError::Store`] when the keychain cannot be updated.
pub fn delete_password(username: &str) -> Result<(), AppError> {
    let entry =
        keyring::Entry::new(KEYRING_SERVICE, username).map_err(|error| keychain_error(&error))?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(keychain_error(&error)),
    }
}

fn keychain_error(error: &keyring::Error) -> AppError {
    AppError::Store(format!("OS keychain error: {error}"))
}
