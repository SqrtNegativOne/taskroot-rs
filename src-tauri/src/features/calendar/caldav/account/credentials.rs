//! Injectable credential storage. Profile IDs isolate usernames across servers.

use crate::error::AppError;

const KEYRING_SERVICE: &str = "taskroot.caldav.profiles";
pub const PASSWORD_ENV: &str = "TASKROOT_CALDAV_PASSWORD";

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

/// OS keychain credentials with a process-level `TASKROOT_CALDAV_PASSWORD`
/// override shared by both desktop and CLI runs.
pub struct KeychainCredentials;

impl CredentialStore for KeychainCredentials {
    fn load(&self, id: &str) -> Result<Option<String>, AppError> {
        load_with_override(std::env::var(PASSWORD_ENV).ok(), || {
            let entry = entry(id)?;
            match entry.get_password() {
                Ok(password) => Ok(Some(password)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(error) => Err(keychain_error(&error)),
            }
        })
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

fn load_with_override<F>(
    env_password: Option<String>,
    fallback: F,
) -> Result<Option<String>, AppError>
where
    F: FnOnce() -> Result<Option<String>, AppError>,
{
    if let Some(password) = env_password.filter(|value| !value.is_empty()) {
        return Ok(Some(password));
    }
    fallback()
}

fn entry(id: &str) -> Result<keyring::Entry, AppError> {
    keyring::Entry::new(KEYRING_SERVICE, id).map_err(|error| keychain_error(&error))
}

fn keychain_error(error: &keyring::Error) -> AppError {
    AppError::Store(format!("OS keychain error: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_empty_env_override_short_circuits_keychain_lookup() {
        let loaded = load_with_override(Some("env-secret".to_string()), || {
            Err(AppError::Store("keychain must not be queried".to_string()))
        })
        .unwrap();

        assert_eq!(loaded.as_deref(), Some("env-secret"));
    }

    #[test]
    fn missing_or_empty_env_override_falls_back_to_store() {
        let from_missing =
            load_with_override(None, || Ok(Some("saved-secret".to_string()))).unwrap();
        assert_eq!(from_missing.as_deref(), Some("saved-secret"));

        let from_empty =
            load_with_override(Some(String::new()), || Ok(Some("saved-secret".to_string())))
                .unwrap();
        assert_eq!(from_empty.as_deref(), Some("saved-secret"));

        let absent = load_with_override(None, || Ok(None)).unwrap();
        assert_eq!(absent, None);
    }
}
