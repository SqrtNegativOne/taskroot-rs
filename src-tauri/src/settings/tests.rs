//! Store-backed settings command tests.

use super::{storage, AppSettings};
use crate::features::calendar::{persist, Store, StoreState};
use serde_json::{json, Value};

#[tokio::test]
async fn get_settings_returns_the_defaults_when_nothing_is_stored() {
    let dir = tempfile::tempdir().unwrap();
    let state = StoreState::new(Store::load(dir.path().to_path_buf()));

    assert_eq!(
        serde_json::to_value(storage::read_settings(&state).await.unwrap()).unwrap(),
        serde_json::to_value(AppSettings::default()).unwrap()
    );
}

#[tokio::test]
async fn update_setting_round_trips_through_the_json_file() {
    let dir = tempfile::tempdir().unwrap();
    let state = StoreState::new(Store::load(dir.path().to_path_buf()));

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
    let state = StoreState::new(Store::load(dir.path().to_path_buf()));

    let error = storage::write_setting(&state, "pause_minutes", &json!("15"))
        .await
        .unwrap_err();

    assert_eq!(error.code(), "invalid-input");
    assert!(!dir.path().join("settings.json").exists());
}

#[tokio::test]
async fn colour_scheme_defaults_to_slate() {
    let dir = tempfile::tempdir().unwrap();
    let state = StoreState::new(Store::load(dir.path().to_path_buf()));
    let settings = serde_json::to_value(storage::read_settings(&state).await.unwrap()).unwrap();

    assert_eq!(settings["colour_scheme"], "slate");
}

#[tokio::test]
async fn colour_scheme_choices_survive_reopening_the_store() {
    let dir = tempfile::tempdir().unwrap();
    let state = StoreState::new(Store::load(dir.path().to_path_buf()));
    let meta = AppSettings::setting_metadata()
        .into_iter()
        .find(|meta| meta.id == "colour_scheme")
        .expect("colour scheme setting must be available in Settings");

    for option in meta.options.expect("theme choices") {
        storage::write_setting(&state, "colour_scheme", &option.value)
            .await
            .unwrap();
        let reopened = StoreState::new(Store::load(dir.path().to_path_buf()));
        let settings = serde_json::to_value(storage::read_settings(&reopened).await.unwrap()).unwrap();
        assert_eq!(settings["colour_scheme"], option.value);
    }
}

#[tokio::test]
async fn invalid_colour_scheme_does_not_change_saved_settings() {
    let dir = tempfile::tempdir().unwrap();
    let state = StoreState::new(Store::load(dir.path().to_path_buf()));
    storage::write_setting(&state, "colour_scheme", &json!("ember"))
        .await
        .unwrap();
    let path = dir.path().join("settings.json");
    let saved = std::fs::read_to_string(&path).unwrap();

    for invalid in [json!("not-a-theme"), json!(42), Value::Null] {
        let error = storage::write_setting(&state, "colour_scheme", &invalid)
            .await
            .unwrap_err();
        assert_eq!(error.code(), "invalid-input");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), saved);
        let settings = serde_json::to_value(storage::read_settings(&state).await.unwrap()).unwrap();
        assert_eq!(settings["colour_scheme"], "ember");
    }
}

#[tokio::test]
async fn ui_state_round_trips_and_is_absent_until_written() {
    let dir = tempfile::tempdir().unwrap();
    let state = StoreState::new(Store::load(dir.path().to_path_buf()));
    let stored = json!({
        "operator": "and", "children": [{
            "kind": "group", "operator": "or", "children": [
                { "kind": "condition", "filter": { "column": "status", "operator": "is", "value": ["todo"] } },
                { "kind": "condition", "filter": { "column": "status", "operator": "is", "value": ["next-up"] } }
            ]
        }]
    });

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

#[tokio::test]
async fn earliest_wake_up_time_defaults_to_zero_and_round_trips_through_settings_json() {
    let dir = tempfile::tempdir().unwrap();
    let state = StoreState::new(Store::load(dir.path().to_path_buf()));

    assert_eq!(
        storage::read_settings(&state).await.unwrap().earliest_wake_up_time,
        0
    );

    storage::write_setting(&state, "earliest_wake_up_time", &json!(360))
        .await
        .unwrap();

    assert_eq!(
        storage::read_settings(&state).await.unwrap().earliest_wake_up_time,
        360
    );
    assert_eq!(persist::read_settings(dir.path()).earliest_wake_up_time, 360);

    let reopened = StoreState::new(Store::load(dir.path().to_path_buf()));
    assert_eq!(
        storage::read_settings(&reopened)
            .await
            .unwrap()
            .earliest_wake_up_time,
        360
    );
}
