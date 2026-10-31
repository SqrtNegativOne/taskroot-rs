//! Store-backed settings command tests.

use super::{storage, AppSettings};
use crate::features::calendar::{persist, Store, StoreState};
use serde_json::{json, Value};

#[tokio::test]
async fn get_settings_returns_the_defaults_when_nothing_is_stored() {
    let dir = tempfile::tempdir().unwrap();
    let state = StoreState::new(Store::load(dir.path().to_path_buf()).expect("load store"));

    assert_eq!(
        serde_json::to_value(storage::read_settings(&state).await.unwrap()).unwrap(),
        serde_json::to_value(AppSettings::default()).unwrap()
    );
}

#[tokio::test]
async fn update_setting_round_trips_through_the_json_file() {
    let dir = tempfile::tempdir().unwrap();
    let state = StoreState::new(Store::load(dir.path().to_path_buf()).unwrap());

    storage::write_setting(&state, "clock_style", &json!("counter"))
        .await
        .unwrap();

    assert_eq!(
        storage::read_settings(&state).await.unwrap().clock_style,
        "counter"
    );
    assert_eq!(
        persist::read_settings(dir.path()).clock_style,
        "counter",
        "the write must reach settings.json"
    );
}

#[tokio::test]
async fn update_setting_rejects_a_wrongly_typed_known_setting_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    let state = StoreState::new(Store::load(dir.path().to_path_buf()).unwrap());

    let error = storage::write_setting(&state, "pause_minutes", &json!("15"))
        .await
        .unwrap_err();

    assert_eq!(error.code(), "invalid-input");
    assert!(!dir.path().join("settings.json").exists());
}

#[tokio::test]
async fn ui_state_round_trips_and_is_absent_until_written() {
    let dir = tempfile::tempdir().unwrap();
    let state = StoreState::new(Store::load(dir.path().to_path_buf()).expect("load store"));
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
