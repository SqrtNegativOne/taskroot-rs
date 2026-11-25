//! Password-free registry persistence, injectable for account lifecycle tests.

use std::collections::BTreeSet;
use std::path::PathBuf;

use super::{account_path, AccountRegistry};
use crate::error::AppError;

pub trait AccountRepository: Send + Sync {
    /// # Errors
    /// Returns a storage error for unreadable or invalid account data.
    fn read(&self) -> Result<AccountRegistry, AppError>;
    /// # Errors
    /// Returns a storage error if the registry cannot be atomically replaced.
    fn write(&self, registry: &AccountRegistry) -> Result<(), AppError>;
}

pub struct JsonAccountRepository {
    dir: PathBuf,
}

impl JsonAccountRepository {
    #[must_use]
    pub const fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
}

impl AccountRepository for JsonAccountRepository {
    fn read(&self) -> Result<AccountRegistry, AppError> {
        let path = account_path(&self.dir);
        let raw = match std::fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(AccountRegistry::default())
            }
            Err(error) => {
                return Err(AppError::Store(format!(
                    "Could not read account file `{}`: {error}",
                    path.display()
                )))
            }
        };
        let registry: AccountRegistry = serde_json::from_str(&raw).map_err(|error| {
            AppError::Store(format!(
                "Malformed account file `{}`: {error}. The file holds no password; \
                 fix or delete it to recover. A pre-registry single-account file is not \
                 migrated; convert it by hand or reconnect (the old password may still \
                 be in the keychain under `taskroot.caldav`).",
                path.display()
            ))
        })?;
        validate(&registry)?;
        Ok(registry)
    }

    fn write(&self, registry: &AccountRegistry) -> Result<(), AppError> {
        validate(registry)?;
        let path = account_path(&self.dir);
        let temp = self.dir.join("account.json.tmp");
        let json = serde_json::to_vec_pretty(registry)
            .map_err(|error| AppError::Store(format!("Could not encode accounts: {error}")))?;
        std::fs::write(&temp, json)
            .map_err(|error| AppError::Store(format!("Could not write account file: {error}")))?;
        std::fs::rename(&temp, &path).map_err(|error| {
            let _ = std::fs::remove_file(&temp);
            AppError::Store(format!("Could not replace account file: {error}"))
        })
    }
}

fn validate(registry: &AccountRegistry) -> Result<(), AppError> {
    let mut ids = BTreeSet::new();
    let mut identities = BTreeSet::new();
    for profile in &registry.profiles {
        if profile.id.is_empty()
            || !ids.insert(&profile.id)
            || !identities.insert((&profile.account.server_url, &profile.account.username))
        {
            return Err(AppError::Store(
                "Malformed account file: duplicate or empty profile identity".to_string(),
            ));
        }
    }
    if registry.active_id.is_some() && registry.active().is_none() {
        return Err(AppError::Store(
            "Malformed account file: active account is missing".to_string(),
        ));
    }
    Ok(())
}
