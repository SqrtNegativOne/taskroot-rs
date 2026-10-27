//! Store persistence, snapshot and query tests.

use super::model::Store;
use super::persist;
use super::query;
use crate::domain::{AppTask, AppTaskFilter, AppTaskFilterColumn, FilterOperator, TaskId, TaskPriority};
use serde_json::json;
use std::path::PathBuf;

fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("store-tests")
        .join(format!("{name}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

const EVENT_RAW: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Test//EN\r\nBEGIN:VEVENT\r\nUID:one\r\nDTSTART;VALUE=DATE:20261017\r\nDTEND;VALUE=DATE:20261018\r\nSUMMARY:Standup\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

#[test]
fn settings_round_trip_and_malformed_file_falls_back_to_defaults() {
    let dir = scratch_dir("settings");
    let settings = crate::settings::AppSettings {
        clock_style: "counter".to_string(),
        ..crate::settings::AppSettings::default()
    };
    persist::write_settings(&dir, &settings).unwrap();

    let loaded = persist::read_settings(&dir);
    assert_eq!(loaded.clock_style, "counter");

    std::fs::write(persist::settings_path(&dir), "not json").unwrap();
    let fallback = persist::read_settings(&dir);
    assert_eq!(
        serde_json::to_value(fallback).unwrap(),
        serde_json::to_value(crate::settings::AppSettings::default()).unwrap()
    );
    assert!(dir.join("settings.json.bad").exists());
}

#[test]
fn default_clock_style_is_flowtime() {
    assert_eq!(
        crate::settings::AppSettings::default().clock_style,
        "flowtime"
    );
}

#[test]
fn a_settings_file_without_a_clock_style_uses_the_default() {
    let dir = scratch_dir("clock-style-missing");
    std::fs::write(persist::settings_path(&dir), "{}").unwrap();

    assert_eq!(persist::read_settings(&dir).clock_style, "flowtime");
}

#[test]
fn apply_setting_rejects_a_wrong_type_and_ignores_unknown_keys() {
    let mut settings = crate::settings::AppSettings::default();

    let error = persist::apply_setting(&mut settings, "sync_interval", &json!("15"))
        .expect_err("a string must be rejected for an i32 setting");
    assert_eq!(error.code(), "invalid-input");

    assert!(!persist::apply_setting(&mut settings, "not_a_setting", &json!(1)).unwrap());
}

#[test]
fn ui_state_round_trips_and_keeps_unknown_keys() {
    let dir = scratch_dir("ui");
    let mut state = serde_json::Map::new();
    state.insert("ui.date_grid.filters".to_string(), json!([{ "column": "status" }]));

    persist::write_ui_state(&dir, &state).unwrap();

    let loaded = persist::read_ui_state(&dir);
    assert_eq!(loaded.get("ui.date_grid.filters"), state.get("ui.date_grid.filters"));
}

#[test]
fn snapshot_round_trips_and_each_resource_reparses() {
    let dir = scratch_dir("snapshot");
    let mut store = Store::load(dir.clone()).unwrap();
    store
        .set_resource("/cal/one.ics", "/cal/", Some("\"etag\""), EVENT_RAW)
        .unwrap();
    store.write_snapshot();

    let mut loaded = Store::load(dir).unwrap();
    assert_eq!(loaded.load_snapshot().unwrap(), 1);

    let event = loaded.event("one").expect("event is restored");
    assert_eq!(event.title, "Standup");
    assert_eq!(
        event.remote_id.as_ref().map(|id| id.as_str()),
        Some("/cal/one.ics")
    );
}

#[test]
fn a_malformed_snapshot_resource_is_skipped() {
    let dir = scratch_dir("bad-snapshot");
    let snapshot = json!({
        "broken": { "collection": "/cal/", "etag": null, "raw": "not an ics" },
        "one": { "collection": "/cal/", "etag": null, "raw": EVENT_RAW }
    });
    std::fs::write(
        dir.join("snapshot.json"),
        serde_json::to_string(&snapshot).unwrap(),
    )
    .unwrap();

    let mut store = Store::load(dir).unwrap();
    assert_eq!(store.load_snapshot().unwrap(), 1);
    assert!(store.event("one").is_some());
}

fn task(id: &str, title: &str, priority: TaskPriority) -> AppTask {
    AppTask {
        id: TaskId::from(id),
        title: title.to_string(),
        status: Some(crate::domain::AppTaskStatus::Todo),
        priority: Some(priority),
        tags: None,
        parent_task: None,
        dependencies: None,
        est: None,
        added: None,
        canvas_x: None,
        canvas_y: None,
        on_canvas: None,
        remote_id: None,
        remote_collection_id: None,
        notes: None,
        tabs: None,
        start: None,
        due: None,
        completed_at: None,
        rrule: None,
        geo: None,
        alarms: None,
        timezone: None,
        updated_at: None,
        etag: None,
    }
}

#[test]
fn priority_filter_compares_the_numeric_bucket() {
    let tasks = vec![
        task("a", "Urgent", TaskPriority::Urgent),
        task("b", "Low", TaskPriority::Low),
    ];
    let filtered = query::query_tasks(
        &tasks,
        &[AppTaskFilter {
            column: Some(AppTaskFilterColumn::Priority),
            operator: Some(FilterOperator::Is),
            value: Some(json!([4])),
        }],
        &[],
        "",
    );

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id.as_str(), "a");
}

#[test]
fn empty_filter_values_do_not_match_anything() {
    let tasks = vec![task("a", "Alpha", TaskPriority::Medium)];
    let filtered = query::query_tasks(
        &tasks,
        &[AppTaskFilter {
            column: Some(AppTaskFilterColumn::Status),
            operator: Some(FilterOperator::IsNot),
            value: Some(json!([])),
        }],
        &[],
        "",
    );

    assert_eq!(filtered.len(), 1, "an empty exclusion set filters nothing");
}

/// A full load must leave the collection's sync token behind, otherwise the next
/// pull asks the server for "everything since the beginning" and re-downloads the
/// whole collection — once per restart, and on every wake-up after an offline gap.
#[tokio::test]
async fn a_full_load_keeps_the_sync_token_so_the_next_pull_is_a_delta() {
    use crate::caldav::model::{CalendarComponent, CalendarInfo};
    use crate::caldav::transport::{CalDavClient, HttpClient};
    use super::ops;
    use std::collections::BTreeMap;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const COLLECTION: &str = "/me/2f07f5a4-5fc2-bc5c-b6de-31053f2b5d33/";
    const TOKEN: &str = "http://radicale.org/ns/sync/572734e7eac248e21146e7a7ff811067b02e20bb6f870c6826d4a1e33e559435";

    let server = MockServer::start().await;
    Mock::given(method("REPORT"))
        .and(path(COLLECTION))
        .and(body_string_contains("<d:sync-token/>"))
        .respond_with(
            ResponseTemplate::new(207)
                .set_body_string(include_str!("../caldav/fixtures/sync_collection_initial.xml")),
        )
        .mount(&server)
        .await;
    Mock::given(method("REPORT"))
        .and(path(COLLECTION))
        .and(body_string_contains("calendar-multiget"))
        .respond_with(
            ResponseTemplate::new(207)
                .set_body_string(include_str!("../caldav/fixtures/multiget.xml")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let http = HttpClient::new().unwrap();
    let client = CalDavClient::new(&http, &server.uri(), "me", "secret").unwrap();
    let mut store = Store {
        data_dir: scratch_dir("full-load-token"),
        resources: BTreeMap::new(),
        calendars: vec![CalendarInfo {
            href: COLLECTION.to_string(),
            display_name: "SqrtCal".to_string(),
            component: CalendarComponent::VEvent,
            ..CalendarInfo::default()
        }],
        tokens: BTreeMap::new(),
        settings: crate::settings::AppSettings::default(),
        ui_state: serde_json::Map::new(),
        offline: false,
        last_synced: None,
    };

    ops::full_load(&mut store, &client).await.unwrap();
    assert_eq!(
        store.tokens.get(COLLECTION).map(String::as_str),
        Some(TOKEN)
    );

    Mock::given(method("REPORT"))
        .and(path(COLLECTION))
        .and(body_string_contains(TOKEN))
        .respond_with(
            ResponseTemplate::new(207)
                .set_body_string(include_str!("../caldav/fixtures/sync_collection_delta.xml")),
        )
        .mount(&server)
        .await;

    ops::pull(&mut store, &client).await.unwrap();
    server.verify().await;
}
