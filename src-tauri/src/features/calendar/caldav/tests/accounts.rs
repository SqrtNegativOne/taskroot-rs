//! Account behavior contracts; credentials never touch the OS keychain.

use std::sync::Arc;

use wiremock::matchers::method;
use wiremock::{Mock, ResponseTemplate};

use crate::features::calendar::{HttpClient, Store, StoreState};

use super::super::account::{
    AccountRepository, AccountService, CredentialStore, JsonAccountRepository,
};
use super::account_support::{server, MemoryCredentials};

fn service(dir: &std::path::Path, credentials: Arc<MemoryCredentials>) -> AccountService {
    AccountService::new(
        Arc::new(JsonAccountRepository::new(dir.to_path_buf())),
        credentials,
        HttpClient::new().unwrap(),
    )
}

#[tokio::test]
async fn same_username_on_two_servers_keeps_both_passwords_and_switches_without_reentry() {
    let dir = tempfile::tempdir().unwrap();
    let credentials = Arc::new(MemoryCredentials::default());
    let service = service(dir.path(), credentials.clone());
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
    let registry = service.registry().await.unwrap();
    let second_id = registry.active_id.unwrap();
    assert_ne!(first_id, second_id);
    assert_eq!(registry.profiles.len(), 2);
    assert_eq!(
        store
            .read(|store| store.events().first().unwrap().title.clone())
            .await,
        "second sample"
    );
    assert_eq!(
        credentials.load(&first_id).unwrap().as_deref(),
        Some("first")
    );
    assert_eq!(
        credentials.load(&second_id).unwrap().as_deref(),
        Some("second")
    );

    service.activate(&store, &first_id).await.unwrap();
    assert_eq!(
        store
            .read(|store| store.events().first().unwrap().title.clone())
            .await,
        "first sample"
    );
    assert_eq!(
        service.registry().await.unwrap().active_id.as_deref(),
        Some(first_id.as_str())
    );
    service.disconnect(&store).await.unwrap();
    assert!(
        store
            .read(|store| store.calendars.is_empty() && store.resources.is_empty())
            .await
    );
    assert_eq!(service.registry().await.unwrap().profiles.len(), 2);
    service.activate(&store, &second_id).await.unwrap();
    service.remove(&store, &first_id).await.unwrap();
    assert_eq!(
        service.registry().await.unwrap().active_id.as_deref(),
        Some(second_id.as_str())
    );
    assert_eq!(credentials.load(&first_id).unwrap(), None);
    assert_eq!(
        credentials.load(&second_id).unwrap().as_deref(),
        Some("second")
    );

    let raw = std::fs::read_to_string(dir.path().join("account.json")).unwrap();
    assert!(!raw.contains("password"));
    assert!(!raw.contains("second\""));
    assert_eq!(
        JsonAccountRepository::new(dir.path().to_path_buf())
            .read()
            .unwrap()
            .profiles
            .len(),
        1
    );
}

#[tokio::test]
async fn failed_switch_keeps_the_active_account_and_loaded_data() {
    let dir = tempfile::tempdir().unwrap();
    let credentials = Arc::new(MemoryCredentials::default());
    let service = service(dir.path(), credentials);
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
    let calendars = store.read(|store| store.calendars.clone()).await;
    store
        .write(|store| {
            store
                .set_resource(
                    "/sentinel.ics",
                    &calendars.first().unwrap().href,
                    Some("etag"),
                    crate::test_support::ics::all_day(),
                )
                .unwrap();
        })
        .await;
    Mock::given(method("REPORT"))
        .respond_with(ResponseTemplate::new(500))
        .with_priority(1)
        .mount(&first)
        .await;
    assert!(service.activate(&store, &first_id).await.is_err());
    assert_eq!(service.registry().await.unwrap(), before);
    assert_eq!(store.read(|store| store.calendars.clone()).await, calendars);
    assert!(
        store
            .read(|store| store.resources.contains_key("/sentinel.ics"))
            .await
    );
}

#[tokio::test]
async fn reconnect_updates_a_profile_and_unknown_ids_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let service = service(dir.path(), Arc::new(MemoryCredentials::default()));
    let store = StoreState::new(Store::load(dir.path().to_path_buf()));
    let server = server("first").await;
    service
        .connect(&store, &server.uri(), "me", "first")
        .await
        .unwrap();
    let id = service.registry().await.unwrap().active_id.unwrap();
    service
        .connect(&store, &format!("{}/", server.uri()), "me", "first")
        .await
        .unwrap();
    assert_eq!(service.registry().await.unwrap().profiles.len(), 1);
    assert_eq!(
        service.registry().await.unwrap().active_id.as_deref(),
        Some(id.as_str())
    );
    assert_eq!(
        service
            .activate(&store, "missing")
            .await
            .unwrap_err()
            .code(),
        "not-found"
    );
    assert_eq!(
        service.remove(&store, "missing").await.unwrap_err().code(),
        "not-found"
    );
}

#[tokio::test]
async fn an_in_flight_client_lease_prevents_switching_accounts() {
    let dir = tempfile::tempdir().unwrap();
    let service = Arc::new(service(dir.path(), Arc::new(MemoryCredentials::default())));
    let store = Arc::new(StoreState::new(Store::load(dir.path().to_path_buf())));
    let server = server("first").await;
    service
        .connect(&store, &server.uri(), "me", "first")
        .await
        .unwrap();
    let lease = service.client().await.unwrap();
    let other_service = service.clone();
    let other_store = store.clone();
    let mut transition = tokio::spawn(async move { other_service.disconnect(&other_store).await });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), &mut transition)
            .await
            .is_err()
    );
    drop(lease);
    transition.await.unwrap().unwrap();
    assert_eq!(service.client().await.err().unwrap().code(), "auth");
}
