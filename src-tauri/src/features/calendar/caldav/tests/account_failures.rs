//! Failure boundaries must preserve both the session and existing credentials.

use super::super::account::{
    AccountRegistry, AccountRepository, AccountService, CredentialStore, JsonAccountRepository,
};
use super::account_support::{server, MemoryCredentials};
use crate::error::AppError;
use crate::features::calendar::{HttpClient, Store, StoreState};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

struct FailingRepository {
    inner: JsonAccountRepository,
    fail: AtomicBool,
}

impl AccountRepository for FailingRepository {
    fn read(&self) -> Result<AccountRegistry, AppError> {
        self.inner.read()
    }
    fn write(&self, registry: &AccountRegistry) -> Result<(), AppError> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(AppError::Store("disk full".to_string()));
        }
        self.inner.write(registry)
    }
}

#[tokio::test]
async fn failed_persistence_does_not_forget_an_account_or_leak_new_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let repository = Arc::new(FailingRepository {
        inner: JsonAccountRepository::new(dir.path().to_path_buf()),
        fail: AtomicBool::new(false),
    });
    let credentials = Arc::new(MemoryCredentials::default());
    let service = AccountService::new(
        repository.clone(),
        credentials.clone(),
        HttpClient::new().unwrap(),
    );
    let store = StoreState::new(Store::load(dir.path().to_path_buf()));
    let first = server("first").await;
    let second = server("second").await;
    service
        .connect(&store, &first.uri(), "me", "first")
        .await
        .unwrap();
    let before = service.registry().await.unwrap();
    let passwords = credentials.0.lock().unwrap().clone();
    let calendars = store.read(|store| store.calendars.clone()).await;
    repository.fail.store(true, Ordering::SeqCst);

    assert_eq!(
        service
            .connect(&store, &second.uri(), "me", "second")
            .await
            .unwrap_err()
            .code(),
        "store"
    );
    assert_eq!(service.registry().await.unwrap(), before);
    assert_eq!(*credentials.0.lock().unwrap(), passwords);
    assert_eq!(store.read(|store| store.calendars.clone()).await, calendars);

    let id = before.active_id.unwrap();
    assert_eq!(
        service.remove(&store, &id).await.unwrap_err().code(),
        "store"
    );
    assert_eq!(credentials.load(&id).unwrap().as_deref(), Some("first"));
    assert_eq!(
        service.registry().await.unwrap().active_id.as_deref(),
        Some(id.as_str())
    );
    assert_eq!(
        service.disconnect(&store).await.unwrap_err().code(),
        "store"
    );
    assert_eq!(
        service.registry().await.unwrap().active_id.as_deref(),
        Some(id.as_str())
    );
}

#[tokio::test]
async fn missing_credentials_fail_without_an_environment_or_other_profile_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let credentials = Arc::new(MemoryCredentials::default());
    let service = AccountService::new(
        Arc::new(JsonAccountRepository::new(dir.path().to_path_buf())),
        credentials.clone(),
        HttpClient::new().unwrap(),
    );
    let store = StoreState::new(Store::load(dir.path().to_path_buf()));
    let first = server("first").await;
    let second = server("second").await;
    service
        .connect(&store, &first.uri(), "me", "first")
        .await
        .unwrap();
    let first_id = service.registry().await.unwrap().active_id.unwrap();
    service
        .connect(&store, &second.uri(), "me", "second")
        .await
        .unwrap();
    let before = service.registry().await.unwrap();
    credentials.delete(&first_id).unwrap();
    assert_eq!(
        service
            .activate(&store, &first_id)
            .await
            .unwrap_err()
            .code(),
        "auth"
    );
    assert_eq!(service.registry().await.unwrap(), before);
}
