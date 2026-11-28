//! Behavioral contracts for field patches, 412 conflict merges, lock-free I/O,
//! and revision/ETag-guarded store updates.

use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::features::calendar::caldav::client::CalDavClient;
use crate::features::calendar::caldav::model::{CalendarComponent, CalendarInfo};
use crate::features::calendar::caldav::transport::HttpClient;
use crate::features::calendar::domain::{
    apply_event_patches, apply_task_patches, AppEvent, AppTask, AppTaskStatus, Color, EventId,
    EventPatch, EventStatus, TaskId, TaskPatch, TaskPriority,
};
use crate::test_support::caldav::start_server;

use super::model::Store;
use super::ops;
use super::state::StoreState;

const TASK_COLLECTION: &str = "/cal/tasks/";
const EVENT_COLLECTION: &str = "/cal/events/";

fn state_with_calendars() -> Arc<StoreState> {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::load(dir.path().to_path_buf());
    store.calendars = vec![
        CalendarInfo {
            href: TASK_COLLECTION.to_string(),
            display_name: "Tasks".to_string(),
            component: CalendarComponent::VTodo,
            writable: true,
            ..CalendarInfo::default()
        },
        CalendarInfo {
            href: EVENT_COLLECTION.to_string(),
            display_name: "Events".to_string(),
            component: CalendarComponent::VEvent,
            writable: true,
            ..CalendarInfo::default()
        },
    ];
    Arc::new(StoreState::new(store))
}

fn sample_task() -> AppTask {
    AppTask {
        id: TaskId::from("t1"),
        title: "Pay rent".to_string(),
        status: Some(AppTaskStatus::Done),
        priority: Some(TaskPriority::High),
        tags: None,
        parent_task: None,
        dependencies: None,
        est: Some(30),
        time_spent: None,
        added: None,
        remote_id: None,
        remote_collection_id: None,
        notes: Some("Keep me".to_string()),
        start: None,
        due: Some("2026-10-20".parse().unwrap()),
        completed_at: Some("2026-10-19T12:00:00Z".parse().unwrap()),
        rrule: None,
        geo: None,
        alarms: None,
        timezone: None,
        updated_at: None,
        etag: None,
    }
}

fn sample_event() -> AppEvent {
    AppEvent {
        id: EventId::from("e1"),
        remote_id: None,
        remote_collection_id: None,
        task_id: None,
        title: "Standup".to_string(),
        description: Some("Old agenda".to_string()),
        priority: Some(TaskPriority::Medium),
        tags: None,
        start_time: "2026-10-20T09:00:00".parse().unwrap(),
        end_time: "2026-10-20T09:30:00".parse().unwrap(),
        rrule: None,
        exdates: None,
        recurring_event_id: None,
        original_start_time: None,
        status: Some(EventStatus::Confirmed),
        updated_at: None,
        added: None,
        color: Some(Color::try_from("#4285f4".to_string()).unwrap()),
        etag: None,
        is_all_day: Some(false),
        timezone: Some("UTC".to_string()),
    }
}

#[test]
fn task_patch_distinguishes_absent_from_cleared_and_clears_completed_at_when_reopened() {
    let mut task = sample_task();

    apply_task_patches(
        &mut task,
        &[
            TaskPatch::Title("Pay October rent".to_string()),
            TaskPatch::Notes(None),
            TaskPatch::Status(Some(AppTaskStatus::Todo)),
        ],
    );

    assert_eq!(task.title, "Pay October rent");
    assert_eq!(task.notes, None, "explicit Notes(None) must clear notes");
    assert_eq!(
        task.due,
        Some("2026-10-20".parse().unwrap()),
        "omitted Due must stay untouched"
    );
    assert_eq!(
        task.priority,
        Some(TaskPriority::High),
        "omitted Priority must stay untouched"
    );
    assert_eq!(
        task.completed_at, None,
        "transitioning away from Done clears completed_at when CompletedAt is not patched"
    );

    let wire = serde_json::to_value(TaskPatch::Notes(None)).unwrap();
    assert_eq!(wire, serde_json::json!({ "field": "notes", "value": null }));
    let roundtrip: TaskPatch = serde_json::from_value(wire).unwrap();
    assert_eq!(roundtrip, TaskPatch::Notes(None));
}

#[test]
fn task_time_spent_patch_round_trips_applies_and_clears() {
    let mut task = sample_task();
    apply_task_patches(&mut task, &[TaskPatch::TimeSpent(Some(1_845))]);
    assert_eq!(task.time_spent, Some(1_845));

    apply_task_patches(&mut task, &[TaskPatch::TimeSpent(None)]);
    assert_eq!(task.time_spent, None);

    let wire = serde_json::to_value(TaskPatch::TimeSpent(Some(90))).unwrap();
    assert_eq!(wire, serde_json::json!({ "field": "timeSpent", "value": 90 }));
    let roundtrip: TaskPatch = serde_json::from_value(wire).unwrap();
    assert_eq!(roundtrip, TaskPatch::TimeSpent(Some(90)));
}

#[test]
fn event_patch_distinguishes_absent_from_cleared() {
    let mut event = sample_event();

    apply_event_patches(
        &mut event,
        &[
            EventPatch::Title("Daily Standup".to_string()),
            EventPatch::Description(None),
        ],
    );

    assert_eq!(event.title, "Daily Standup");
    assert_eq!(
        event.description, None,
        "explicit Description(None) must clear description"
    );
    assert_eq!(
        event.color,
        Some(Color::try_from("#4285f4".to_string()).unwrap()),
        "omitted Color must stay untouched"
    );

    let wire = serde_json::to_value(EventPatch::IsAllDay(Some(true))).unwrap();
    assert_eq!(
        wire,
        serde_json::json!({ "field": "isAllDay", "value": true })
    );
}

#[tokio::test]
async fn task_412_conflict_merges_only_patched_fields_onto_refetched_resource() {
    const HREF: &str = "/cal/tasks/t1.ics";
    const INITIAL: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VTODO\r\nUID:t1\r\nSUMMARY:Pay rent\r\nDESCRIPTION:Initial note\r\nDUE;VALUE=DATE:20261020\r\nSTATUS:NEEDS-ACTION\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";
    const REMOTE_V2: &str = "BEGIN:VCALENDAR\nVERSION:2.0\nBEGIN:VTODO\nUID:t1\nSUMMARY:Pay rent\nDESCRIPTION:Updated from phone\nDUE;VALUE=DATE:20261025\nSTATUS:NEEDS-ACTION\nEND:VTODO\nEND:VCALENDAR\n";

    let (server, client) = start_server().await;
    Mock::given(method("PUT"))
        .and(path(HREF))
        .and(header("if-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(412))
        .expect(1)
        .mount(&server)
        .await;
    let multiget = format!(
        "<?xml version='1.0' encoding='utf-8'?><multistatus xmlns=\"DAV:\" \
         xmlns:C=\"urn:ietf:params:xml:ns:caldav\"><response><href>{HREF}</href>\
         <propstat><prop><getetag>\"v2\"</getetag><C:calendar-data>{REMOTE_V2}</C:calendar-data></prop>\
         <status>HTTP/1.1 200 OK</status></propstat></response></multistatus>"
    );
    Mock::given(method("REPORT"))
        .and(path(TASK_COLLECTION))
        .respond_with(ResponseTemplate::new(207).set_body_string(multiget))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path(HREF))
        .and(header("if-match", "\"v2\""))
        .respond_with(ResponseTemplate::new(204).insert_header("ETag", "\"v3\""))
        .expect(1)
        .mount(&server)
        .await;

    let state = state_with_calendars();
    state
        .write(|store| store.set_resource(HREF, TASK_COLLECTION, Some("\"v1\""), INITIAL))
        .await
        .unwrap();

    ops::update_task(
        &state,
        &client,
        "t1",
        &[TaskPatch::Title("Pay October rent".to_string())],
    )
    .await
    .unwrap();

    let (task, raw, etag) = state
        .read(|store| {
            let res = store.resources.get(HREF).unwrap();
            (
                store.task("t1").unwrap().clone(),
                res.raw.clone(),
                res.etag.clone(),
            )
        })
        .await;
    assert_eq!(etag.as_deref(), Some("\"v3\""));
    assert_eq!(task.title, "Pay October rent");
    assert_eq!(
        task.notes.as_deref(),
        Some("Updated from phone"),
        "concurrent phone edit to notes must survive 412 merge: {raw}"
    );
    assert_eq!(
        task.due,
        Some("2026-10-25".parse().unwrap()),
        "concurrent phone edit to due must survive 412 merge: {raw}"
    );

    let requests = server.received_requests().await.unwrap();
    let retry_put = requests
        .iter()
        .find(|req| {
            req.method == "PUT"
                && req
                    .headers
                    .get("if-match")
                    .and_then(|v| v.to_str().ok())
                    == Some("\"v2\"")
        })
        .expect("retry PUT with fresh ETag must be sent");
    let body = String::from_utf8_lossy(&retry_put.body);
    assert!(body.contains("SUMMARY:Pay October rent"), "{body}");
    assert!(body.contains("DESCRIPTION:Updated from phone"), "{body}");
    assert!(body.contains("DUE;VALUE=DATE:20261025"), "{body}");
}

#[tokio::test]
async fn event_412_conflict_merges_only_patched_fields_onto_refetched_resource() {
    const HREF: &str = "/cal/events/e1.ics";
    const INITIAL: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:e1\r\nSUMMARY:Standup\r\nDESCRIPTION:Old agenda\r\nDTSTART:20261020T090000Z\r\nDTEND:20261020T093000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    const REMOTE_V2: &str = "BEGIN:VCALENDAR\nVERSION:2.0\nBEGIN:VEVENT\nUID:e1\nSUMMARY:Standup\nDESCRIPTION:Phone agenda\nCOLOR:#dc143c\nDTSTART:20261020T090000Z\nDTEND:20261020T093000Z\nEND:VEVENT\nEND:VCALENDAR\n";

    let (server, client) = start_server().await;
    Mock::given(method("PUT"))
        .and(path(HREF))
        .and(header("if-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(412))
        .expect(1)
        .mount(&server)
        .await;
    let multiget = format!(
        "<?xml version='1.0' encoding='utf-8'?><multistatus xmlns=\"DAV:\" \
         xmlns:C=\"urn:ietf:params:xml:ns:caldav\"><response><href>{HREF}</href>\
         <propstat><prop><getetag>\"v2\"</getetag><C:calendar-data>{REMOTE_V2}</C:calendar-data></prop>\
         <status>HTTP/1.1 200 OK</status></propstat></response></multistatus>"
    );
    Mock::given(method("REPORT"))
        .and(path(EVENT_COLLECTION))
        .respond_with(ResponseTemplate::new(207).set_body_string(multiget))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path(HREF))
        .and(header("if-match", "\"v2\""))
        .respond_with(ResponseTemplate::new(204).insert_header("ETag", "\"v3\""))
        .expect(1)
        .mount(&server)
        .await;

    let state = state_with_calendars();
    state
        .write(|store| store.set_resource(HREF, EVENT_COLLECTION, Some("\"v1\""), INITIAL))
        .await
        .unwrap();

    ops::update_event(
        &state,
        &client,
        "e1",
        &[EventPatch::Title("Daily Standup".to_string())],
    )
    .await
    .unwrap();

    let (event, raw, etag) = state
        .read(|store| {
            let res = store.resources.get(HREF).unwrap();
            (
                store.event("e1").unwrap().clone(),
                res.raw.clone(),
                res.etag.clone(),
            )
        })
        .await;
    assert_eq!(etag.as_deref(), Some("\"v3\""));
    assert_eq!(event.title, "Daily Standup");
    assert_eq!(
        event.description.as_deref(),
        Some("Phone agenda"),
        "concurrent phone edit to description must survive 412 merge: {raw}"
    );
    assert_eq!(
        event.color.as_ref().map(|c| c.0.as_str()),
        Some("#dc143c"),
        "concurrent phone edit to color must survive 412 merge: {raw}"
    );

    let requests = server.received_requests().await.unwrap();
    let retry_put = requests
        .iter()
        .find(|req| {
            req.method == "PUT"
                && req
                    .headers
                    .get("if-match")
                    .and_then(|v| v.to_str().ok())
                    == Some("\"v2\"")
        })
        .expect("retry PUT with fresh ETag must be sent");
    let body = String::from_utf8_lossy(&retry_put.body);
    assert!(body.contains("SUMMARY:Daily Standup"), "{body}");
    assert!(body.contains("DESCRIPTION:Phone agenda"), "{body}");
    assert!(body.contains("COLOR:#dc143c"), "{body}");
}

/// Spawn a one-shot HTTP server that accepts one request, signals `entered`,
/// waits on `release`, and replies `204 No Content` with `response_etag`.
async fn spawn_stalling_server(
    response_etag: &'static str,
) -> (
    CalDavClient,
    oneshot::Receiver<()>,
    oneshot::Sender<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (entered_tx, entered_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();

    tokio::spawn(async move {
        let Ok((mut socket, _)) = listener.accept().await else {
            return;
        };
        let mut buf = [0u8; 4096];
        let _ = socket.read(&mut buf).await;
        let _ = entered_tx.send(());
        let _ = release_rx.await;
        let response = format!(
            "HTTP/1.1 204 No Content\r\nETag: {response_etag}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let _ = socket.write_all(response.as_bytes()).await;
    });

    let http = HttpClient::new().unwrap();
    let client = CalDavClient::new(&http, &format!("http://{addr}"), "u", "p").unwrap();
    (client, entered_rx, release_tx)
}

async fn assert_store_readable_while_in_flight(state: &StoreState) {
    let settings = tokio::time::timeout(
        Duration::from_millis(250),
        state.read(|store| store.settings.clone()),
    )
    .await
    .expect("store lock must not be held during network I/O");
    assert_ne!(settings.clock_style, "");
}

#[tokio::test]
async fn writes_do_not_hold_store_lock_across_network_io() {
    // 1. create_task
    {
        let (client, entered, release) = spawn_stalling_server("\"c1\"").await;
        let state = state_with_calendars();
        let state_bg = Arc::clone(&state);
        let mut task = sample_task();
        task.id = TaskId::from("new-task");
        let handle = tokio::spawn(async move { ops::create_task(&state_bg, &client, task).await });

        entered.await.unwrap();
        assert_store_readable_while_in_flight(&state).await;
        release.send(()).unwrap();
        handle.await.unwrap().unwrap();
        assert!(state.read(|s| s.task("new-task").is_some()).await);
    }

    // 2. update_task
    {
        let (client, entered, release) = spawn_stalling_server("\"v2\"").await;
        let state = state_with_calendars();
        let initial = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VTODO\r\nUID:t1\r\nSUMMARY:Old\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";
        state
            .write(|s| s.set_resource("/cal/tasks/t1.ics", TASK_COLLECTION, Some("\"v1\""), initial))
            .await
            .unwrap();
        let state_bg = Arc::clone(&state);
        let handle = tokio::spawn(async move {
            ops::update_task(&state_bg, &client, "t1", &[TaskPatch::Title("New".into())]).await
        });

        entered.await.unwrap();
        assert_store_readable_while_in_flight(&state).await;
        release.send(()).unwrap();
        handle.await.unwrap().unwrap();
        assert_eq!(
            state
                .read(|s| s.task("t1").map(|t| t.title.clone()))
                .await
                .as_deref(),
            Some("New")
        );
    }

    // 3. delete_task
    {
        let (client, entered, release) = spawn_stalling_server("\"d1\"").await;
        let state = state_with_calendars();
        let initial = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VTODO\r\nUID:t1\r\nSUMMARY:Bye\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";
        state
            .write(|s| s.set_resource("/cal/tasks/t1.ics", TASK_COLLECTION, Some("\"v1\""), initial))
            .await
            .unwrap();
        let state_bg = Arc::clone(&state);
        let handle = tokio::spawn(async move { ops::delete_task(&state_bg, &client, "t1").await });

        entered.await.unwrap();
        assert_store_readable_while_in_flight(&state).await;
        release.send(()).unwrap();
        handle.await.unwrap().unwrap();
        assert!(state.read(|s| s.task("t1").is_none()).await);
    }

    // 4. update_event
    {
        let (client, entered, release) = spawn_stalling_server("\"e2\"").await;
        let state = state_with_calendars();
        let initial = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:e1\r\nSUMMARY:Old\r\nDTSTART:20261020T090000Z\r\nDTEND:20261020T093000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        state
            .write(|s| {
                s.set_resource(
                    "/cal/events/e1.ics",
                    EVENT_COLLECTION,
                    Some("\"e1\""),
                    initial,
                )
            })
            .await
            .unwrap();
        let state_bg = Arc::clone(&state);
        let handle = tokio::spawn(async move {
            ops::update_event(
                &state_bg,
                &client,
                "e1",
                &[EventPatch::Title("Updated".into())],
            )
            .await
        });

        entered.await.unwrap();
        assert_store_readable_while_in_flight(&state).await;
        release.send(()).unwrap();
        handle.await.unwrap().unwrap();
        assert_eq!(
            state
                .read(|s| s.event("e1").map(|e| e.title.clone()))
                .await
                .as_deref(),
            Some("Updated")
        );
    }
}

#[tokio::test]
async fn in_flight_write_does_not_clobber_newer_etag_in_store() {
    const HREF: &str = "/cal/tasks/t1.ics";
    const V1: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VTODO\r\nUID:t1\r\nSUMMARY:V1\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";
    const V3: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VTODO\r\nUID:t1\r\nSUMMARY:Newer V3\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";

    let (client, entered, release) = spawn_stalling_server("\"v2\"").await;
    let state = state_with_calendars();
    state
        .write(|s| s.set_resource(HREF, TASK_COLLECTION, Some("\"v1\""), V1))
        .await
        .unwrap();

    let state_bg = Arc::clone(&state);
    let handle = tokio::spawn(async move {
        ops::update_task(
            &state_bg,
            &client,
            "t1",
            &[TaskPatch::Title("Stale V2".into())],
        )
        .await
    });

    entered.await.unwrap();
    // Simulate a newer write/refresh landing while the PUT was in flight.
    state
        .write(|s| s.set_resource(HREF, TASK_COLLECTION, Some("\"v3\""), V3))
        .await
        .unwrap();

    release.send(()).unwrap();
    handle.await.unwrap().unwrap();

    let (title, etag) = state
        .read(|s| {
            (
                s.task("t1").unwrap().title.clone(),
                s.resources.get(HREF).unwrap().etag.clone(),
            )
        })
        .await;
    assert_eq!(etag.as_deref(), Some("\"v3\""));
    assert_eq!(
        title, "Newer V3",
        "older in-flight write must not overwrite a newer ETag"
    );
}

#[tokio::test]
async fn committed_write_supersedes_concurrent_load_with_older_etag() {
    const HREF: &str = "/cal/tasks/t1.ics";
    const V1: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VTODO\r\nUID:t1\r\nSUMMARY:Old\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";

    let (client, entered, release) = spawn_stalling_server("\"v2\"").await;
    let state = state_with_calendars();
    state
        .write(|s| s.set_resource(HREF, TASK_COLLECTION, Some("\"v1\""), V1))
        .await
        .unwrap();

    let state_bg = Arc::clone(&state);
    let handle = tokio::spawn(async move {
        ops::update_task(
            &state_bg,
            &client,
            "t1",
            &[TaskPatch::Title("Committed New".into())],
        )
        .await
    });

    entered.await.unwrap();
    // Simulate a full_load that started before our PUT and re-applied the same "v1" ETag
    // (which bumps store.revision()).
    state
        .write(|s| s.set_resource(HREF, TASK_COLLECTION, Some("\"v1\""), V1))
        .await
        .unwrap();

    release.send(()).unwrap();
    handle.await.unwrap().unwrap();

    let (title, etag) = state
        .read(|s| {
            (
                s.task("t1").unwrap().title.clone(),
                s.resources.get(HREF).unwrap().etag.clone(),
            )
        })
        .await;
    assert_eq!(etag.as_deref(), Some("\"v2\""));
    assert_eq!(
        title, "Committed New",
        "server-confirmed PUT must supersede an older load snapshot with the matching prior ETag"
    );
}
