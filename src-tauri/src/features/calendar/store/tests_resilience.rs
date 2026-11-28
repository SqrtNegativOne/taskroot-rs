//! Wiremock regression tests for scoped-delete 412 retry, cross-collection move
//! rollback/retry, and `EventScope::All` reschedule from exception rows.

use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::features::calendar::caldav::client::CalDavClient;
use crate::features::calendar::caldav::model::{CalendarComponent, CalendarInfo};
use crate::features::calendar::caldav::transport::HttpClient;
use crate::features::calendar::domain::{CollectionId, EventPatch, EventScope, TaskPatch};

use super::model::Store;
use super::ops;
use super::state::StoreState;

const EVENTS_A: &str = "/cal/events/";
const EVENTS_B: &str = "/cal/events-2/";
const TASKS_A: &str = "/cal/tasks/";
const TASKS_B: &str = "/cal/tasks-2/";

fn calendar(href: &str, name: &str, component: CalendarComponent) -> CalendarInfo {
    CalendarInfo {
        href: href.to_string(),
        display_name: name.to_string(),
        component,
        writable: true,
        ..CalendarInfo::default()
    }
}

fn state_with_pairs() -> StoreState {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::load(dir.path().to_path_buf());
    store.set_calendars(vec![
        calendar(EVENTS_A, "Events A", CalendarComponent::VEvent),
        calendar(EVENTS_B, "Events B", CalendarComponent::VEvent),
        calendar(TASKS_A, "Tasks A", CalendarComponent::VTodo),
        calendar(TASKS_B, "Tasks B", CalendarComponent::VTodo),
    ]);
    StoreState::new(store)
}

async fn start_server() -> (MockServer, CalDavClient) {
    let server = MockServer::start().await;
    let http = HttpClient::new().unwrap();
    let client = CalDavClient::new(&http, &server.uri(), "me", "pw").unwrap();
    (server, client)
}

fn multiget_xml(href: &str, etag: &str, ics: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:response>
    <d:href>{href}</d:href>
    <d:propstat>
      <d:prop>
        <d:getetag>{etag}</d:getetag>
        <c:calendar-data>{ics}</c:calendar-data>
      </d:prop>
      <d:status>HTTP/1.1 200 OK</d:status>
    </d:propstat>
  </d:response>
</d:multistatus>"#
    )
}

#[tokio::test]
async fn scoped_delete_put_rewritten_retries_412_and_preserves_concurrent_edit() {
    const HREF: &str = "/cal/events/series.ics";
    const V1: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:series\r\nSUMMARY:Standup\r\nDTSTART;TZID=Asia/Kolkata:20261019T110000\r\nDTEND;TZID=Asia/Kolkata:20261019T113000\r\nRRULE:FREQ=WEEKLY\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    const V2: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:series\r\nSUMMARY:Renamed Standup\r\nDTSTART;TZID=Asia/Kolkata:20261019T110000\r\nDTEND;TZID=Asia/Kolkata:20261019T113000\r\nRRULE:FREQ=WEEKLY\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

    let (server, client) = start_server().await;
    Mock::given(method("PUT"))
        .and(path(HREF))
        .and(header("if-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(412))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("REPORT"))
        .and(path(EVENTS_A))
        .respond_with(ResponseTemplate::new(207).set_body_string(multiget_xml(HREF, "\"v2\"", V2)))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path(HREF))
        .and(header("if-match", "\"v2\""))
        .respond_with(ResponseTemplate::new(204).insert_header("etag", "\"v3\""))
        .expect(1)
        .mount(&server)
        .await;

    let state = state_with_pairs();
    state
        .write(|s| s.set_resource(HREF, EVENTS_A, Some("\"v1\""), V1))
        .await
        .unwrap();

    ops::delete_event(
        &state,
        &client,
        "series",
        EventScope::This,
        Some("2026-10-26T11:00:00".parse().unwrap()),
    )
    .await
    .unwrap();

    let (etag, title, raw) = state
        .read(|s| {
            let res = s.resources.get(HREF).unwrap();
            (
                res.etag.clone(),
                s.event("series").unwrap().title.clone(),
                res.raw.clone(),
            )
        })
        .await;
    assert_eq!(etag.as_deref(), Some("\"v3\""));
    assert_eq!(title, "Renamed Standup");
    assert!(raw.contains("EXDATE;TZID=Asia/Kolkata:20261026T110000"), "{raw}");
}

#[tokio::test]
async fn move_rolls_back_destination_when_source_delete_fails() {
    const OLD_HREF: &str = "/cal/tasks/t1.ics";
    const NEW_HREF: &str = "/cal/tasks-2/t1.ics";
    const V1: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VTODO\r\nUID:t1\r\nSUMMARY:Move me\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";

    let (server, client) = start_server().await;
    Mock::given(method("PUT"))
        .and(path(NEW_HREF))
        .and(header("if-none-match", "*"))
        .respond_with(ResponseTemplate::new(201).insert_header("etag", "\"dest-v1\""))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(OLD_HREF))
        .and(header("if-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(NEW_HREF))
        .and(header("if-match", "\"dest-v1\""))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let state = state_with_pairs();
    state
        .write(|s| s.set_resource(OLD_HREF, TASKS_A, Some("\"v1\""), V1))
        .await
        .unwrap();

    let err = ops::update_task(
        &state,
        &client,
        "t1",
        &[TaskPatch::RemoteCollectionId(Some(CollectionId::from(
            TASKS_B,
        )))],
    )
    .await
    .unwrap_err();
    assert_eq!(err.code(), "remote");

    // Store still has the task only in its original collection.
    state
        .read(|s| {
            assert!(s.resources.contains_key(OLD_HREF));
            assert!(!s.resources.contains_key(NEW_HREF));
        })
        .await;
}

#[tokio::test]
async fn move_retries_412_on_source_delete_and_commits_move() {
    const OLD_HREF: &str = "/cal/events/e1.ics";
    const NEW_HREF: &str = "/cal/events-2/e1.ics";
    const V1: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:e1\r\nSUMMARY:Move event\r\nDTSTART:20261019T090000Z\r\nDTEND:20261019T100000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

    let (server, client) = start_server().await;
    Mock::given(method("PUT"))
        .and(path(NEW_HREF))
        .and(header("if-none-match", "*"))
        .respond_with(ResponseTemplate::new(201).insert_header("etag", "\"dest-v1\""))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(OLD_HREF))
        .and(header("if-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(412))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("REPORT"))
        .and(path(EVENTS_A))
        .respond_with(
            ResponseTemplate::new(207).set_body_string(multiget_xml(OLD_HREF, "\"v2\"", V1)),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(OLD_HREF))
        .and(header("if-match", "\"v2\""))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let state = state_with_pairs();
    state
        .write(|s| s.set_resource(OLD_HREF, EVENTS_A, Some("\"v1\""), V1))
        .await
        .unwrap();

    ops::update_event(
        &state,
        &client,
        "e1",
        &[EventPatch::RemoteCollectionId(Some(CollectionId::from(
            EVENTS_B,
        )))],
    )
    .await
    .unwrap();

    state
        .read(|s| {
            assert!(!s.resources.contains_key(OLD_HREF));
            assert_eq!(
                s.resources.get(NEW_HREF).and_then(|r| r.etag.as_deref()),
                Some("\"dest-v1\"")
            );
        })
        .await;
}

#[tokio::test]
async fn reschedule_all_from_exception_row_updates_master_instead_of_exception() {
    const HREF: &str = "/cal/events/series.ics";
    const SERIES_WITH_EXCEPTION: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:series\r\nSUMMARY:Standup\r\nDTSTART;TZID=Asia/Kolkata:20261019T110000\r\nDTEND;TZID=Asia/Kolkata:20261019T113000\r\nRRULE:FREQ=WEEKLY\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:series\r\nRECURRENCE-ID;TZID=Asia/Kolkata:20261026T110000\r\nSUMMARY:Standup\r\nDTSTART;TZID=Asia/Kolkata:20261026T140000\r\nDTEND;TZID=Asia/Kolkata:20261026T143000\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

    let (server, client) = start_server().await;
    Mock::given(method("PUT"))
        .and(path(HREF))
        .respond_with(ResponseTemplate::new(204).insert_header("etag", "\"v2\""))
        .mount(&server)
        .await;

    let state = state_with_pairs();
    state
        .write(|s| s.set_resource(HREF, EVENTS_A, Some("\"v1\""), SERIES_WITH_EXCEPTION))
        .await
        .unwrap();

    let exception_id = state
        .read(|s| {
            s.events()
                .into_iter()
                .find(|event| event.recurring_event_id.is_some())
                .unwrap()
                .id
                .to_string()
        })
        .await;

    // Following on an exception row is still rejected as unsupported.
    let following_err = ops::reschedule_event(
        &state,
        &client,
        ops::Reschedule {
            id: exception_id.clone(),
            start_time: "2026-10-19T09:30:00Z".parse().unwrap(),
            end_time: "2026-10-19T10:00:00Z".parse().unwrap(),
            is_all_day: false,
            scope: EventScope::Following,
            occurrence_start: Some("2026-10-26T11:00:00".parse().unwrap()),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(following_err.code(), "invalid-input");

    // Scope::All invoked with the exception id resolves and moves the master.
    ops::reschedule_event(
        &state,
        &client,
        ops::Reschedule {
            id: exception_id.clone(),
            start_time: "2026-10-19T09:30:00Z".parse().unwrap(),
            end_time: "2026-10-19T10:00:00Z".parse().unwrap(),
            is_all_day: false,
            scope: EventScope::All,
            occurrence_start: Some("2026-10-26T11:00:00".parse().unwrap()),
        },
    )
    .await
    .unwrap();

    let (raw, master_start, exception_start) = state
        .read(|s| {
            (
                s.resources.get(HREF).unwrap().raw.clone(),
                s.event("series").unwrap().start_time.to_string(),
                s.event(&exception_id).unwrap().start_time.to_string(),
            )
        })
        .await;
    assert!(raw.contains("DTSTART;TZID=Asia/Kolkata:20261019T150000"), "{raw}");
    assert!(raw.contains("RRULE:FREQ=WEEKLY"), "{raw}");
    assert_eq!(master_start, "2026-10-19T15:00:00");
    assert_eq!(
        exception_start, "2026-10-26T14:00:00",
        "master should be updated, not the exception row"
    );
}
