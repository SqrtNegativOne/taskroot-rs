//! Store-backed settings command tests.

use super::{storage, AppSettings};
use crate::store::{persist, Store, StoreState};
use serde_json::{json, Value};
use std::path::PathBuf;

fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("settings-tests")
        .join(format!("{name}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn state(name: &str) -> StoreState {
    StoreState::new(Store::load(scratch_dir(name)).expect("load store"))
}

#[tokio::test]
async fn get_settings_returns_the_defaults_when_nothing_is_stored() {
    let state = state("defaults");

    assert_eq!(
        serde_json::to_value(storage::read_settings(&state).await.unwrap()).unwrap(),
        serde_json::to_value(AppSettings::default()).unwrap()
    );
}

#[tokio::test]
async fn update_setting_round_trips_through_the_json_file() {
    let dir = scratch_dir("round-trip");
    let state = StoreState::new(Store::load(dir.clone()).unwrap());

    storage::write_setting(&state, "clock_style", &json!("counter"))
        .await
        .unwrap();

    assert_eq!(
        storage::read_settings(&state).await.unwrap().clock_style,
        "counter"
    );
    assert_eq!(
        persist::read_settings(&dir).clock_style,
        "counter",
        "the write must reach settings.json"
    );
}

#[tokio::test]
async fn update_setting_rejects_a_wrongly_typed_known_setting_without_writing() {
    let dir = scratch_dir("reject");
    let state = StoreState::new(Store::load(dir.clone()).unwrap());

    let error = storage::write_setting(&state, "pause_minutes", &json!("15"))
        .await
        .unwrap_err();

    assert_eq!(error.code(), "invalid-input");
    assert!(!dir.join("settings.json").exists());
}

#[tokio::test]
async fn ui_state_round_trips_and_is_absent_until_written() {
    let state = state("ui");
    let stored = json!([{ "column": "status", "value": ["done"] }]);

    assert_eq!(
        storage::read_ui_state(&state, "ui.task_list.filters")
            .await
            .unwrap(),
        Value::Null
    );

    storage::write_ui_state(&state, "ui.task_list.filters", &stored)
        .await
        .unwrap();

    assert_eq!(
        storage::read_ui_state(&state, "ui.task_list.filters")
            .await
            .unwrap(),
        stored
    );
    assert_eq!(
        storage::all_ui_state(&state)
            .await
            .get("ui.task_list.filters"),
        Some(&stored)
    );
}
