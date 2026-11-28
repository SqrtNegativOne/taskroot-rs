//! Account lifecycle and session isolation, independent of Tauri.

use std::sync::Arc;

use tokio::sync::RwLock;

use super::super::discovery;
use super::{Account, AccountProfile, AccountRegistry, AccountRepository, CredentialStore};
use crate::error::AppError;
use crate::features::calendar::{apply, ops, CalDavClient, Fetched, HttpClient, Store, StoreState};

pub struct AccountService {
    pub(super) repository: Arc<dyn AccountRepository>,
    pub(super) credentials: Arc<dyn CredentialStore>,
    http: HttpClient,
    pub(super) gate: Arc<RwLock<()>>,
}

impl AccountService {
    #[must_use]
    pub fn new(
        repository: Arc<dyn AccountRepository>,
        credentials: Arc<dyn CredentialStore>,
        http: HttpClient,
    ) -> Self {
        Self {
            repository,
            credentials,
            http,
            gate: Arc::new(RwLock::new(())),
        }
    }

    /// # Errors
    /// Returns registry storage failures.
    pub async fn registry(&self) -> Result<AccountRegistry, AppError> {
        let _guard = self.gate.read().await;
        self.repository.read()
    }

    /// Discover without persisting or activating anything.
    /// # Errors
    /// Returns invalid-input, authentication or transport failures.
    pub async fn test(
        &self,
        server: &str,
        username: &str,
        password: &str,
    ) -> Result<Account, AppError> {
        if username.trim().is_empty() || password.is_empty() {
            return Err(AppError::InvalidInput(
                "Username and password are required".to_string(),
            ));
        }
        let found = discovery::discover(&self.http, server, username, password).await?;
        Ok(Account {
            server_url: found.server_url,
            username: username.to_string(),
            principal_href: found.principal_href,
            home_href: found.home_href,
            calendars: found.calendars,
        })
    }

    /// Add/reconnect a profile and activate only after its full fetch succeeds.
    /// # Errors
    /// Returns discovery, fetch, credential or persistence failures.
    pub async fn connect(
        &self,
        store: &StoreState,
        server: &str,
        username: &str,
        password: &str,
    ) -> Result<Account, AppError> {
        let _guard = self.gate.write().await;
        let account = self.test(server, username, password).await?;
        let client = CalDavClient::from_account(&self.http, &account, password)?;
        let fetched = fetch(&client, &account).await?;
        let mut registry = self.repository.read()?;
        let id = registry
            .profiles
            .iter()
            .find(|profile| {
                profile.account.server_url == account.server_url
                    && profile.account.username == account.username
            })
            .map_or_else(
                || uuid::Uuid::new_v4().to_string(),
                |profile| profile.id.clone(),
            );
        let previous = self.credentials.load(&id)?;
        self.credentials.save(&id, password)?;
        registry.profiles.retain(|profile| profile.id != id);
        registry.profiles.push(AccountProfile {
            id: id.clone(),
            account: account.clone(),
        });
        registry.active_id = Some(id.clone());
        let mut store = store.lock().await;
        if let Err(error) = self.repository.write(&registry) {
            self.restore_password(&id, previous.as_deref());
            return Err(error);
        }
        apply(&mut store, fetched);
        Ok(account)
    }

    /// Switch in place. The old account/data stay intact on any fetch failure.
    /// # Errors
    /// Returns missing-profile, credential, fetch or storage failures.
    pub async fn activate(&self, store: &StoreState, id: &str) -> Result<(), AppError> {
        let _guard = self.gate.write().await;
        let mut registry = self.repository.read()?;
        if registry.active_id.as_deref() == Some(id) {
            return Ok(());
        }
        let profile = registry.profile(id)?.clone();
        let client = self.profile_client(&profile)?;
        let calendars = discovery::fetch_calendars(&client, &profile.account.home_href).await?;
        let mut account = profile.account;
        account.calendars = calendars;
        let fetched = fetch(&client, &account).await?;
        let profile = registry
            .profiles
            .iter_mut()
            .find(|profile| profile.id == id)
            .ok_or_else(no_account)?;
        profile.account = account;
        registry.active_id = Some(id.to_string());
        let mut store = store.lock().await;
        self.repository.write(&registry)?;
        apply(&mut store, fetched);
        drop(store);
        Ok(())
    }

    /// Fetch outside the store lock, guarding against both writes and switches.
    /// # Errors
    /// Returns account, credential, fetch or persistence failures.
    pub async fn refresh(&self, state: &StoreState) -> Result<(), AppError> {
        let _guard = self.gate.read().await;
        let mut registry = self.repository.read()?;
        let profile = registry.active().ok_or_else(no_account)?.clone();
        let client = self.profile_client(&profile)?;
        let revision = state.read(Store::revision).await;
        let mut account = profile.account;
        account.calendars =
            discovery::refresh_calendar_list(&client, &account.home_href, &account.calendars)
                .await?;
        let fetched = fetch(&client, &account).await?;
        let mut store = state.lock().await;
        if store.revision() != revision {
            return Ok(());
        }
        if let Some(active) = registry
            .profiles
            .iter_mut()
            .find(|candidate| candidate.id == profile.id)
        {
            active.account = account;
        }
        self.repository.write(&registry)?;
        apply(&mut store, fetched);
        drop(store);
        Ok(())
    }

    pub(super) fn profile_client(
        &self,
        profile: &AccountProfile,
    ) -> Result<CalDavClient, AppError> {
        let password = self.credentials.load(&profile.id)?.ok_or_else(|| {
            AppError::Auth(format!(
                "The saved password is missing; reconnect this account or set {}",
                super::PASSWORD_ENV
            ))
        })?;
        CalDavClient::from_account(&self.http, &profile.account, &password)
    }

    pub(super) fn restore_password(&self, id: &str, previous: Option<&str>) {
        let result = previous.map_or_else(
            || self.credentials.delete(id),
            |password| self.credentials.save(id, password),
        );
        if let Err(error) = result {
            eprintln!("Could not roll back account credentials: {error}");
        }
    }
}

pub(super) fn no_account() -> AppError {
    AppError::Auth("No CalDAV account is active".to_string())
}

async fn fetch(client: &CalDavClient, account: &Account) -> Result<Fetched, AppError> {
    let collections = ops::fetch_collections(client, &account.calendars).await?;
    Ok(Fetched {
        calendars: account.calendars.clone(),
        collections,
    })
}
