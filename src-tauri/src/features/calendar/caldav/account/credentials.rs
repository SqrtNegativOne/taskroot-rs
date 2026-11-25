//! Injectable credential storage. Profile IDs isolate usernames across servers.

use crate::error::AppError;

const KEYRING_SERVICE: &str = "taskroot.caldav.profiles";

pub trait CredentialStore: Send + Sync {
    /// # Errors
    /// Returns a storage error when the credential backend cannot be queried.
    fn load(&self, id: &str) -> Result<Option<String>, AppError>;
    /// # Errors
    /// Returns a storage error when the credential backend rejects the write.
    fn save(&self, id: &str, password: &str) -> Result<(), AppError>;
    /// # Errors
    /// Returns a storage error when deletion fails. Missing credentials succeed.
    fn delete(&self, id: &str) -> Result<(), AppError>;
}

pub struct KeychainCredentials;

impl CredentialStore for KeychainCredentials {
    fn load(&self, id: &str) -> Result<Option<String>, AppError> {
        let entry = entry(id)?;
        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(keychain_error(&error)),
        }
    }

    fn save(&self, id: &str, password: &str) -> Result<(), AppError> {
        entry(id)?
            .set_password(password)
            .map_err(|error| keychain_error(&error))
    }

    fn delete(&self, id: &str) -> Result<(), AppError> {
        match entry(id)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(keychain_error(&error)),
        }
    }
}

fn entry(id: &str) -> Result<keyring::Entry, AppError> {
    keyring::Entry::new(KEYRING_SERVICE, id).map_err(|error| keychain_error(&error))
}

fn keychain_error(error: &keyring::Error) -> AppError {
    AppError::Store(format!("OS keychain error: {error}"))
}
