//! Store persistence and query tests.

use super::model::Store;
use super::persist;
use super::query;
use crate::features::calendar::domain::FilterGroup;
use crate::features::calendar::domain::{
    AppTask, AppTaskFilter, AppTaskFilterColumn, EventScope, FilterOperator, TaskId, TaskPriority,
};
use serde_json::json;

use crate::features::calendar::caldav::client::CalDavClient;
use crate::features::calendar::caldav::model::{CalendarComponent, CalendarInfo};
use crate::features::calendar::occurrences::expand_event_instances;
use crate::test_support::caldav::start_server;
use crate::test_support::ics::{all_day, series, with_exception};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SERIES_HREF: &str = "/cal/series.ics";
const SERIES_COLLECTION: &str = "/cal/";

fn store_with_series(raw: &str) -> Store {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::load(dir.path().to_path_buf());
    store.calendars = vec![CalendarInfo {
        href: SERIES_COLLECTION.to_string(),
        display_name: "Cal".to_string(),
        component: CalendarComponent::VEvent,
        ..CalendarInfo::default()
    }];
    store
        .set_resource(SERIES_HREF, SERIES_COLLECTION, Some("\"etag\""), raw)
        .unwrap();
    store
}

async fn editing_server() -> (MockServer, CalDavClient) {
    let (server, client) = start_server().await;
    Mock::given(method("PUT"))
        .and(path(SERIES_HREF))
        .respond_with(ResponseTemplate::new(204).insert_header("ETag", "\"next\""))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    (server, client)
}

fn occurrence_starts(store: &Store, start: &str, end: &str) -> Vec<String> {
    expand_event_instances(&store.events(), start, end)
        .into_iter()
        .map(|instance| instance.occurrence_start)
        .collect()
}

/// Every timed instance's UTC start, sorted, so an exception's moved slot is
/// comparable regardless of expansion order.
fn timed_starts(store: &Store, start: &str, end: &str) -> Vec<String> {
    let mut starts: Vec<String> = expand_event_instances(&store.events(), start, end)
        .into_iter()
        .filter_map(|instance| match instance.timing {
            crate::features::calendar::domain::EventTiming::Timed { start, .. } => {
                Some(start.to_rfc3339())
            }
            crate::features::calendar::domain::EventTiming::AllDay { .. } => None,
        })
        .collect();
    starts.sort();
    starts
}

const OCTOBER_RANGE: (&str, &str) = ("2026-10-19T00:00:00Z", "2026-11-02T23:59:59Z");

const TASK_WITH_TAGS: &str = include_str!("../ics/fixtures/tasks-org-task-alarm.ics");

#[test]
fn tag_names_are_deduped_and_sorted_across_events_and_tasks() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::load(dir.path().to_path_buf());
    store
        .set_resource("/cal/event.ics", "/cal/", None, all_day())
        .unwrap();
    store
        .set_resource("/cal/task.ics", "/cal/", None, TASK_WITH_TAGS)
        .unwrap();

    assert_eq!(store.tag_names(), vec!["Garden", "Holiday", "Home"]);
}

fn series_raw(store: &Store) -> &str {
    &store.resource_for_event("series").unwrap().raw
}

#[test]
fn settings_round_trip_and_partial_or_malformed_files_fall_back_to_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path();
    let settings = crate::settings::AppSettings {
        clock_style: "counter".to_string(),
        ..crate::settings::AppSettings::default()
    };
    persist::write_settings(dir, &settings).unwrap();

    let loaded = persist::read_settings(dir);
    assert_eq!(loaded.clock_style, "counter");

    std::fs::write(persist::settings_path(dir), r#"{"pause_minutes":20}"#).unwrap();
    let partial = persist::read_settings(dir);
    assert_eq!(partial.pause_minutes, 20, "the stored field must be kept");
    assert_eq!(
        serde_json::to_value(partial).unwrap(),
        serde_json::to_value(crate::settings::AppSettings {
            pause_minutes: 20,
            ..crate::settings::AppSettings::default()
        })
        .unwrap(),
        "fields the file omits must be completed from the defaults"
    );

    std::fs::write(persist::settings_path(dir), "not json").unwrap();
    let fallback = persist::read_settings(dir);
    assert_eq!(
        serde_json::to_value(fallback).unwrap(),
        serde_json::to_value(crate::settings::AppSettings::default()).unwrap()
    );
    assert!(dir.join("settings.json.bad").exists());
}

#[test]
fn apply_setting_rejects_a_wrong_type_and_ignores_unknown_keys() {
    let mut settings = crate::settings::AppSettings::default();

    let error = persist::apply_setting(&mut settings, "pause_minutes", &json!("15"))
        .expect_err("a string must be rejected for an i32 setting");
    assert_eq!(error.code(), "invalid-input");

    assert!(!persist::apply_setting(&mut settings, "not_a_setting", &json!(1)).unwrap());
}

#[test]
fn ui_state_round_trips_and_keeps_unknown_keys() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path();
    let mut state = serde_json::Map::new();
    state.insert(
        "ui.date_grid.filters".to_string(),
        json!([{ "column": "status" }]),
    );

    persist::write_ui_state(dir, &state).unwrap();

    let loaded = persist::read_ui_state(dir);
    assert_eq!(
        loaded.get("ui.date_grid.filters"),
        state.get("ui.date_grid.filters")
    );
}

#[tokio::test]
async fn full_load_replaces_the_store_with_the_server_contents() {
    use super::ops;
    use wiremock::matchers::body_string_contains;

    const COLLECTION: &str = "/me/2f07f5a4-5fc2-bc5c-b6de-31053f2b5d33/";

    let (server, client) = start_server().await;
    let raw = all_day()
        .replace(
            "UID:dsh-20261017-first-day-of-durga@dsh-import",
            "UID:fresh",
        )
        .replace("\r\n", "\n");
    let body = format!(
        "<?xml version='1.0' encoding='utf-8'?><multistatus xmlns=\"DAV:\" \
         xmlns:C=\"urn:ietf:params:xml:ns:caldav\"><response><href>{COLLECTION}fresh.ics</href>\
         <propstat><prop><getetag>\"etag\"</getetag><C:calendar-data>{raw}</C:calendar-data></prop>\
         <status>HTTP/1.1 200 OK</status></propstat></response></multistatus>",
    );
    Mock::given(method("REPORT"))
        .and(path(COLLECTION))
        .and(body_string_contains("<c:calendar-query"))
        .respond_with(ResponseTemplate::new(207).set_body_string(body))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::load(dir.path().to_path_buf());
    store
        .set_resource("/cal/stale.ics", "/cal/", None, all_day())
        .unwrap();
    store.calendars = vec![CalendarInfo {
        href: COLLECTION.to_string(),
        display_name: "SqrtCal".to_string(),
        component: CalendarComponent::VEvent,
        ..CalendarInfo::default()
    }];

    ops::full_load(&mut store, &client).await.unwrap();

    assert_eq!(store.resources.len(), 1, "stale resources are dropped");
    assert!(store
        .event("dsh-20261017-first-day-of-durga@dsh-import")
        .is_none());
    assert_eq!(
        store.event("fresh").map(|event| event.title.as_str()),
        Some("First Day of Durga")
    );
}

#[tokio::test]
async fn deleting_one_occurrence_excludes_it_and_keeps_the_rule() {
    use super::ops;

    let (_server, client) = editing_server().await;
    let mut store = store_with_series(series());

    ops::delete_event(
        &mut store,
        &client,
        "series",
        EventScope::This,
        Some("2026-10-26T11:00:00"),
    )
    .await
    .unwrap();

    let raw = series_raw(&store);
    assert!(
        raw.contains("EXDATE;TZID=Asia/Kolkata:20261026T110000"),
        "the occurrence must be excluded in the event's zone: {raw}"
    );
    assert!(
        raw.contains("RRULE:FREQ=WEEKLY"),
        "the rule is untouched: {raw}"
    );
    assert_eq!(
        occurrence_starts(&store, OCTOBER_RANGE.0, OCTOBER_RANGE.1),
        vec!["2026-10-19T11:00:00", "2026-11-02T11:00:00"],
        "only the excluded occurrence disappears"
    );
}

#[tokio::test]
async fn deleting_this_and_following_truncates_the_rule_and_drops_later_exceptions() {
    use super::ops;

    let (_server, client) = editing_server().await;
    let mut store = store_with_series(&with_exception(series()));

    ops::delete_event(
        &mut store,
        &client,
        "series",
        EventScope::Following,
        Some("2026-10-26T11:00:00"),
    )
    .await
    .unwrap();

    let raw = series_raw(&store);
    assert!(
        raw.contains("UNTIL=20261026T052959Z"),
        "the rule ends one second before 11:00 IST: {raw}"
    );
    assert!(
        !raw.contains("RECURRENCE-ID"),
        "an exception after the cutoff must not survive: {raw}"
    );
    assert_eq!(
        occurrence_starts(&store, OCTOBER_RANGE.0, OCTOBER_RANGE.1),
        vec!["2026-10-19T11:00:00"]
    );
}

#[tokio::test]
async fn deleting_all_removes_the_whole_resource_when_an_exception_was_selected() {
    use super::ops;

    let (_server, client) = editing_server().await;
    let mut store = store_with_series(&with_exception(series()));

    ops::delete_event(
        &mut store,
        &client,
        "series@2026-11-02T11:00:00",
        EventScope::All,
        None,
    )
    .await
    .unwrap();

    assert!(store.resources.is_empty(), "the series resource is gone");
}

#[tokio::test]
async fn rescheduling_one_occurrence_writes_an_exception_and_leaves_the_series() {
    use super::ops;

    let (_server, client) = editing_server().await;
    let mut store = store_with_series(series());

    ops::reschedule_event(
        &mut store,
        &client,
        ops::Reschedule {
            id: "series".to_string(),
            start_time: "2026-10-26T08:30:00Z".to_string(),
            end_time: "2026-10-26T09:00:00Z".to_string(),
            is_all_day: false,
            scope: EventScope::This,
            occurrence_start: Some("2026-10-26T11:00:00".to_string()),
        },
    )
    .await
    .unwrap();

    let raw = series_raw(&store);
    assert!(
        raw.contains("RECURRENCE-ID;TZID=Asia/Kolkata:20261026T110000"),
        "the moved occurrence becomes an exception: {raw}"
    );
    assert!(
        raw.contains("DTSTART;TZID=Asia/Kolkata:20261026T140000"),
        "the exception keeps the wall clock (08:30Z is 14:00 IST): {raw}"
    );
    assert!(raw.contains("DTSTART;TZID=Asia/Kolkata:20261019T110000"));
    assert_eq!(
        timed_starts(&store, OCTOBER_RANGE.0, OCTOBER_RANGE.1),
        vec![
            "2026-10-19T05:30:00+00:00",
            "2026-10-26T08:30:00+00:00",
            "2026-11-02T05:30:00+00:00"
        ],
        "the moved occurrence replaces its slot and the others keep theirs"
    );
}

#[tokio::test]
async fn rescheduling_the_series_moves_the_master() {
    use super::ops;

    let (_server, client) = editing_server().await;
    let mut store = store_with_series(series());

    ops::reschedule_event(
        &mut store,
        &client,
        ops::Reschedule {
            id: "series".to_string(),
            start_time: "2026-10-19T09:30:00Z".to_string(),
            end_time: "2026-10-19T10:00:00Z".to_string(),
            is_all_day: false,
            scope: EventScope::All,
            occurrence_start: None,
        },
    )
    .await
    .unwrap();

    let raw = series_raw(&store);
    assert!(
        raw.contains("DTSTART;TZID=Asia/Kolkata:20261019T150000"),
        "{raw}"
    );
    assert!(raw.contains("RRULE:FREQ=WEEKLY"), "{raw}");
    assert_eq!(
        occurrence_starts(&store, OCTOBER_RANGE.0, OCTOBER_RANGE.1),
        vec![
            "2026-10-19T15:00:00",
            "2026-10-26T15:00:00",
            "2026-11-02T15:00:00"
        ]
    );
}

#[tokio::test]
async fn moving_a_series_from_one_occurrence_is_refused() {
    use super::ops;

    let (_server, client) = editing_server().await;
    let mut store = store_with_series(series());

    let error = ops::reschedule_event(
        &mut store,
        &client,
        ops::Reschedule {
            id: "series".to_string(),
            start_time: "2026-10-26T08:30:00Z".to_string(),
            end_time: "2026-10-26T09:00:00Z".to_string(),
            is_all_day: false,
            scope: EventScope::Following,
            occurrence_start: Some("2026-10-26T11:00:00".to_string()),
        },
    )
    .await
    .expect_err("rescheduling from an occurrence would shift every earlier one");

    assert_eq!(error.code(), "invalid-input");
}

#[tokio::test]
async fn deleting_this_on_a_non_recurring_event_deletes_it() {
    use super::ops;

    let (_server, client) = editing_server().await;
    let mut store = store_with_series(all_day());

    ops::delete_event(
        &mut store,
        &client,
        "dsh-20261017-first-day-of-durga@dsh-import",
        EventScope::This,
        None,
    )
    .await
    .unwrap();

    assert!(
        store.resources.is_empty(),
        "a one-off event has no occurrence to except; it is removed outright"
    );
}

fn task(id: &str, title: &str, priority: TaskPriority) -> AppTask {
    AppTask {
        id: TaskId::from(id),
        title: title.to_string(),
        status: Some(crate::features::calendar::domain::AppTaskStatus::Todo),
        priority: Some(priority),
        tags: None,
        parent_task: None,
        dependencies: None,
        est: None,
        added: None,
        remote_id: None,
        remote_collection_id: None,
        notes: None,
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
        &FilterGroup::from(vec![AppTaskFilter {
            column: Some(AppTaskFilterColumn::Priority),
            operator: Some(FilterOperator::Is),
            value: Some(json!([4])),
        }]),
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
        &FilterGroup::from(vec![AppTaskFilter {
            column: Some(AppTaskFilterColumn::Status),
            operator: Some(FilterOperator::IsNot),
            value: Some(json!([])),
        }]),
        &[],
        "",
    );

    assert_eq!(filtered.len(), 1, "an empty exclusion set filters nothing");
}

fn calendar(
    href: &str,
    summary: &str,
    component: CalendarComponent,
    writable: bool,
) -> CalendarInfo {
    CalendarInfo {
        href: href.to_string(),
        display_name: summary.to_string(),
        component,
        writable,
        ..CalendarInfo::default()
    }
}

fn store_with_calendars() -> Store {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::load(dir.path().to_path_buf());
    store.calendars = vec![
        calendar("/cal/tasks/", "Tasks", CalendarComponent::VTodo, true),
        calendar("/cal/tasks2/", "Tasks 2", CalendarComponent::VTodo, true),
        calendar(
            "/cal/readonly/",
            "Read-only",
            CalendarComponent::VTodo,
            false,
        ),
        calendar("/cal/events/", "Events", CalendarComponent::VEvent, true),
        calendar("/cal/events2/", "Events 2", CalendarComponent::VEvent, true),
        calendar(
            "/cal/mismatch/",
            "Wrong component",
            CalendarComponent::VEvent,
            true,
        ),
    ];
    store
}

fn default_href(store: &Store, component: CalendarComponent) -> Option<&str> {
    store
        .default_calendar(component, None)
        .map(|calendar| calendar.href.as_str())
}

#[test]
fn default_calendar_uses_each_configured_list() {
    let mut store = store_with_calendars();
    store.settings.default_task_list = "/cal/tasks2/".to_string();
    store.settings.default_event_list = "/cal/events2/".to_string();

    assert_eq!(
        default_href(&store, CalendarComponent::VTodo),
        Some("/cal/tasks2/")
    );
    assert_eq!(
        default_href(&store, CalendarComponent::VEvent),
        Some("/cal/events2/")
    );
}

#[test]
fn default_calendar_falls_back_to_the_first_writable_list() {
    let store = store_with_calendars();

    assert_eq!(
        default_href(&store, CalendarComponent::VTodo),
        Some("/cal/tasks/")
    );
    assert_eq!(
        default_href(&store, CalendarComponent::VEvent),
        Some("/cal/events/")
    );
}

#[test]
fn default_calendar_ignores_a_deleted_wrong_component_or_read_only_default() {
    for configured in ["/cal/missing/", "/cal/mismatch/", "/cal/readonly/"] {
        let mut store = store_with_calendars();
        store.settings.default_task_list = configured.to_string();

        assert_eq!(
            default_href(&store, CalendarComponent::VTodo),
            Some("/cal/tasks/"),
            "{configured} must not be selected for a task"
        );
    }
}

#[test]
fn an_explicit_calendar_request_wins_over_the_configured_default() {
    let mut store = store_with_calendars();
    store.settings.default_task_list = "/cal/tasks/".to_string();

    let selected = store.default_calendar(CalendarComponent::VTodo, Some("/cal/tasks2/"));

    assert_eq!(
        selected.map(|calendar| calendar.href.as_str()),
        Some("/cal/tasks2/")
    );
}

#[test]
fn an_invalid_explicit_calendar_request_falls_through_to_the_configured_default() {
    let mut store = store_with_calendars();
    store.settings.default_task_list = "/cal/tasks2/".to_string();

    let selected = store.default_calendar(CalendarComponent::VTodo, Some("/cal/readonly/"));

    assert_eq!(
        selected.map(|calendar| calendar.href.as_str()),
        Some("/cal/tasks2/")
    );
}

#[tokio::test]
async fn conflict_retry_revalidates_recurring_task_completion_against_refetched_body() {
    use super::ops;
    use crate::features::calendar::domain::AppTaskStatus;

    const TASK_HREF: &str = "/cal/tasks/t1.ics";
    const INITIAL: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VTODO\r\nUID:t1\r\nSUMMARY:Pay rent\r\nSTATUS:NEEDS-ACTION\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";
    const REMOTE_RECURRING: &str = "BEGIN:VCALENDAR\nVERSION:2.0\nBEGIN:VTODO\nUID:t1\nSUMMARY:Pay rent\nSTATUS:NEEDS-ACTION\nRRULE:FREQ=MONTHLY\nEND:VTODO\nEND:VCALENDAR\n";

    let (server, client) = start_server().await;
    Mock::given(method("PUT"))
        .and(path(TASK_HREF))
        .respond_with(ResponseTemplate::new(412))
        .expect(1)
        .mount(&server)
        .await;
    let multiget = format!(
        "<?xml version='1.0' encoding='utf-8'?><multistatus xmlns=\"DAV:\" \
         xmlns:C=\"urn:ietf:params:xml:ns:caldav\"><response><href>{TASK_HREF}</href>\
         <propstat><prop><getetag>\"v2\"</getetag><C:calendar-data>{REMOTE_RECURRING}</C:calendar-data></prop>\
         <status>HTTP/1.1 200 OK</status></propstat></response></multistatus>"
    );
    Mock::given(method("REPORT"))
        .and(path("/cal/tasks/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(multiget))
        .expect(1)
        .mount(&server)
        .await;

    let mut store = store_with_calendars();
    store
        .set_resource(TASK_HREF, "/cal/tasks/", Some("\"v1\""), INITIAL)
        .unwrap();
    let mut updated = store.task("t1").unwrap().clone();
    updated.status = Some(AppTaskStatus::Done);

    let error = ops::update_task(&mut store, &client, updated)
        .await
        .expect_err("a remotely added RRULE must block completion on retry");
    assert_eq!(error.code(), "invalid-input");
}

#[tokio::test]
async fn read_only_source_collections_reject_deletes_and_moves_before_network_io() {
    use super::ops;
    use crate::features::calendar::domain::CollectionId;

    let (_server, client) = start_server().await;
    let mut store = store_with_calendars();
    store.calendars.push(calendar(
        "/cal/ro-events/",
        "Read-only events",
        CalendarComponent::VEvent,
        false,
    ));
    let raw_task = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VTODO\r\nUID:ro-task\r\nSUMMARY:Locked\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";
    store
        .set_resource("/cal/readonly/ro-task.ics", "/cal/readonly/", Some("\"e\""), raw_task)
        .unwrap();
    store
        .set_resource("/cal/ro-events/ev.ics", "/cal/ro-events/", Some("\"e\""), all_day())
        .unwrap();

    assert_eq!(
        ops::delete_task(&mut store, &client, "ro-task")
            .await
            .unwrap_err()
            .code(),
        "invalid-input"
    );
    let mut moved_task = store.task("ro-task").unwrap().clone();
    moved_task.remote_collection_id = Some(CollectionId::from("/cal/tasks/"));
    assert_eq!(
        ops::update_task(&mut store, &client, moved_task)
            .await
            .unwrap_err()
            .code(),
        "invalid-input"
    );

    let event_id = store.events().first().unwrap().id.to_string();
    assert_eq!(
        ops::delete_event(&mut store, &client, &event_id, EventScope::All, None)
            .await
            .unwrap_err()
            .code(),
        "invalid-input"
    );
    let mut moved_event = store.event(&event_id).unwrap().clone();
    moved_event.remote_collection_id = Some(CollectionId::from("/cal/events/"));
    assert_eq!(
        ops::update_event(&mut store, &client, moved_event)
            .await
            .unwrap_err()
            .code(),
        "invalid-input"
    );
}

#[tokio::test]
async fn collection_mutations_bump_store_revision() {
    use crate::features::calendar::caldav::calendars;

    let (server, client) = start_server().await;
    Mock::given(method("PROPPATCH"))
        .and(path("/cal/tasks/"))
        .respond_with(ResponseTemplate::new(207))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/cal/tasks/"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let mut store = store_with_calendars();
    let dir = store.data_dir.clone();
    let before_rename = store.revision();
    calendars::rename(&dir, &client, &mut store, "/cal/tasks/", "Renamed")
        .await
        .unwrap();
    let after_rename = store.revision();
    assert_ne!(after_rename, before_rename);

    calendars::delete(&dir, &client, &mut store, "/cal/tasks/")
        .await
        .unwrap();
    assert_ne!(store.revision(), after_rename);
}
