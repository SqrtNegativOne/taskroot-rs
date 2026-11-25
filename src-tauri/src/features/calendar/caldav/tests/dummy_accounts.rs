//! Explicit opt-in contract against the disposable fixture, never port 5232.

use super::super::account::{AccountService, JsonAccountRepository};
use super::account_support::MemoryCredentials;
use crate::features::calendar::{HttpClient, Store, StoreState};
use std::sync::Arc;

#[tokio::test]
#[ignore = "requires scripts/dummy-caldav.py --check-backend on loopback port 15232"]
async fn disposable_dummy_accounts_switch_real_resources_without_keychain_access() {
    let dir = tempfile::tempdir().unwrap();
    let service = AccountService::new(
        Arc::new(JsonAccountRepository::new(dir.path().to_path_buf())),
        Arc::new(MemoryCredentials::default()),
        HttpClient::new().unwrap(),
    );
    let store = StoreState::new(Store::load(dir.path().to_path_buf()));
    store
        .write(|store| {
            store.settings.default_task_duration = 42;
            store
                .ui_state
                .insert("test.window".to_string(), serde_json::json!({"width": 800}));
        })
        .await;
    let server = "http://127.0.0.1:15232";
    service
        .connect(&store, server, "taskroot-dummy-a", "dummy-a-only")
        .await
        .unwrap();
    let first = service.registry().await.unwrap().active_id.unwrap();
    assert_eq!(
        store
            .read(|store| store.task("dummy-tasks").unwrap().title.clone())
            .await,
        "taskroot-dummy-a sample tasks"
    );
    service
        .connect(&store, server, "taskroot-dummy-b", "dummy-b-only")
        .await
        .unwrap();
    assert_eq!(
        store
            .read(|store| store.task("dummy-tasks").unwrap().title.clone())
            .await,
        "taskroot-dummy-b sample tasks"
    );
    service.activate(&store, &first).await.unwrap();
    assert_eq!(
        store
            .read(|store| store.task("dummy-tasks").unwrap().title.clone())
            .await,
        "taskroot-dummy-a sample tasks"
    );
    assert!(
        store
            .read(|store| store
                .resources
                .values()
                .all(|resource| resource.href.starts_with("/taskroot-dummy-a/")))
            .await
    );
    assert_eq!(
        store
            .read(|store| store.settings.default_task_duration)
            .await,
        42
    );
    assert_eq!(
        store
            .read(|store| store.ui_state.get("test.window").cloned())
            .await,
        Some(serde_json::json!({"width": 800}))
    );
}
