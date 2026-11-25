//! Leases pin account identity throughout a store operation, including reminders.

use super::service::AccountService;
use crate::error::AppError;
use crate::features::calendar::CalDavClient;
use std::ops::Deref;
use tokio::sync::OwnedRwLockReadGuard;

pub struct AccountScope {
    pub id: String,
    _guard: OwnedRwLockReadGuard<()>,
}

/// Keep this lease alive through the entire store/network operation.
/// A switch cannot commit while a command still uses the previous client.
pub struct ClientLease {
    client: CalDavClient,
    _scope: AccountScope,
}

impl Deref for ClientLease {
    type Target = CalDavClient;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl AccountService {
    /// Pin the active identity without querying credentials or contacting a server.
    /// # Errors
    /// Returns missing-account or registry storage failures.
    pub async fn scope(&self) -> Result<AccountScope, AppError> {
        let guard = self.gate.clone().read_owned().await;
        let registry = self.repository.read()?;
        let id = registry
            .active_id
            .ok_or_else(|| AppError::Auth("No CalDAV account is active".to_string()))?;
        Ok(AccountScope { id, _guard: guard })
    }

    /// # Errors
    /// Returns missing-account, credential or URL failures.
    pub async fn client(&self) -> Result<ClientLease, AppError> {
        let scope = self.scope().await?;
        let registry = self.repository.read()?;
        let profile = registry.profile(&scope.id)?;
        let client = self.profile_client(profile)?;
        Ok(ClientLease {
            client,
            _scope: scope,
        })
    }
}
