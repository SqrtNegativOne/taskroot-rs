#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Command-layer smoke tests for the settings commands.
//!
//! Each handler in the parent module resolves the pool and delegates to its
//! `&SqlitePool` body in [`super::storage`], so these tests drive that body
//! against a real in-memory database.

use super::{storage, AppSettings};
use crate::db;
use crate::test_support::in_memory_pool;
use serde_json::{json, Value};

#[tokio::test]
async fn get_settings_returns_the_defaults_when_nothing_is_stored() {
    let pool = in_memory_pool().await;

    assert_eq!(db::get_setting(&pool, "clock_style").await.unwrap(), None);
    let settings = storage::read_settings(&pool).await.unwrap();

    assert_eq!(
        serde_json::to_value(settings).unwrap(),
        serde_json::to_value(AppSettings::default()).unwrap()
    );
}

#[tokio::test]
async fn update_setting_round_trips_through_get_settings() {
    let pool = in_memory_pool().await;

    storage::write_setting(&pool, "clock_style", &json!("counter"))
        .await
        .unwrap();

    assert_eq!(
        storage::read_settings(&pool).await.unwrap().clock_style,
        "counter"
    );
    assert_eq!(
        db::get_setting(&pool, "clock_style").await.unwrap().as_deref(),
        Some("\"counter\"")
    );
}

#[tokio::test]
async fn update_setting_rejects_a_wrongly_typed_known_setting_without_writing() {
    let pool = in_memory_pool().await;

    let error = storage::write_setting(&pool, "sync_interval", &json!("15"))
        .await
        .unwrap_err();

    assert_eq!(error.code(), "invalid-input");
    assert_eq!(db::get_setting(&pool, "sync_interval").await.unwrap(), None);
}

#[tokio::test]
async fn ui_state_round_trips_and_is_absent_until_written() {
    let pool = in_memory_pool().await;
    let stored = json!([{ "column": "status", "value": ["done"] }]);

    assert_eq!(
        storage::read_ui_state(&pool, "ui.task_list.filters")
            .await
            .unwrap(),
        Value::Null
    );

    storage::write_ui_state(&pool, "ui.task_list.filters", &stored)
        .await
        .unwrap();

    assert_eq!(
        storage::read_ui_state(&pool, "ui.task_list.filters")
            .await
            .unwrap(),
        stored
    );
}
