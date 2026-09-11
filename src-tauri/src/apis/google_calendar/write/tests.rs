#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use crate::domain::{CollectionId, EventId, RemoteId};
use serde_json::json;

fn event() -> AppEvent {
    AppEvent {
        id: EventId("local-1".into()),
        remote_id: None,
        remote_collection_id: Some(CollectionId("work cal".into())),
        task_id: None,
        title: "Standup".into(),
        description: Some("Daily sync".into()),
        start_time: "2026-09-02T09:00:00Z".into(),
        end_time: "2026-09-02T09:30:00Z".into(),
        rrule: None,
        exdates: None,
        recurring_event_id: None,
        original_start_time: None,
        status: Some(EventStatus::Confirmed),
        updated_at: None,
        color: None,
        etag: None,
        dirty: Some(true),
        is_all_day: Some(false),
        timezone: None,
    }
}

fn body_field<'a>(write: &'a GoogleEventWrite, key: &str) -> &'a Value {
    write.body.get(key).unwrap()
}

#[test]
fn new_event_is_inserted_with_post() {
    let write = build_write(&event());
    assert_eq!(write.method, Method::POST);
    assert_eq!(
        write.url,
        "https://www.googleapis.com/calendar/v3/calendars/work%20cal/events"
    );
}

#[test]
fn existing_event_is_patched_with_patch() {
    let mut existing = event();
    existing.remote_id = Some(RemoteId("remote-1".into()));

    let write = build_write(&existing);
    assert_eq!(write.method, Method::PATCH);
    assert_eq!(
        write.url,
        "https://www.googleapis.com/calendar/v3/calendars/work%20cal/events/remote-1"
    );
}

#[test]
fn recurring_master_sends_each_recurrence_line() {
    let mut recurring = event();
    recurring.rrule =
        Some("RRULE:FREQ=WEEKLY;BYDAY=MO\nEXDATE;VALUE=DATE:20260914\n".into());
    recurring.remote_id = Some(RemoteId("remote-1".into()));

    let write = build_write(&recurring);

    assert_eq!(
        body_field(&write, "recurrence"),
        &json!(["RRULE:FREQ=WEEKLY;BYDAY=MO", "EXDATE;VALUE=DATE:20260914"])
    );
}

#[test]
fn non_recurring_event_sends_an_empty_recurrence_list() {
    let write = build_write(&event());
    assert_eq!(body_field(&write, "recurrence"), &json!([]));
}

#[test]
fn timed_event_sends_timezone_on_start_and_end() {
    let mut timed = event();
    timed.timezone = Some("America/Los_Angeles".into());

    let write = build_write(&timed);

    assert_eq!(
        body_field(&write, "start"),
        &json!({ "dateTime": "2026-09-02T09:00:00Z", "timeZone": "America/Los_Angeles" })
    );
    assert_eq!(
        body_field(&write, "end"),
        &json!({ "dateTime": "2026-09-02T09:30:00Z", "timeZone": "America/Los_Angeles" })
    );
}

#[test]
fn timed_event_without_timezone_omits_the_field() {
    let write = build_write(&event());
    assert_eq!(
        body_field(&write, "start"),
        &json!({ "dateTime": "2026-09-02T09:00:00Z" })
    );
}

#[test]
fn all_day_event_uses_date_fields_and_strips_the_time() {
    let mut all_day = event();
    all_day.is_all_day = Some(true);
    all_day.start_time = "2026-09-07T00:00:00".into();
    all_day.end_time = "2026-09-08T00:00:00".into();

    let write = build_write(&all_day);

    assert_eq!(body_field(&write, "start"), &json!({ "date": "2026-09-07" }));
    assert_eq!(body_field(&write, "end"), &json!({ "date": "2026-09-08" }));
}

#[test]
fn all_day_event_keeps_timezone_so_recurrence_anchors_correctly() {
    let mut all_day = event();
    all_day.is_all_day = Some(true);
    all_day.start_time = "2026-09-07".into();
    all_day.end_time = "2026-09-08".into();
    all_day.timezone = Some("America/Los_Angeles".into());

    let write = build_write(&all_day);

    assert_eq!(
        body_field(&write, "start"),
        &json!({ "date": "2026-09-07", "timeZone": "America/Los_Angeles" })
    );
}

#[test]
fn status_maps_to_google_wire_values() {
    let mut tentative = event();
    tentative.status = Some(EventStatus::Tentative);
    assert_eq!(body_field(&build_write(&tentative), "status"), "tentative");

    let mut cancelled = event();
    cancelled.status = Some(EventStatus::Cancelled);
    assert_eq!(body_field(&build_write(&cancelled), "status"), "cancelled");

    let mut unspecified = event();
    unspecified.status = None;
    assert_eq!(body_field(&build_write(&unspecified), "status"), "confirmed");
}

#[test]
fn color_is_never_written_back() {
    let mut colored = event();
    colored.color = Some(crate::domain::Color("#a4bdfc".into()));

    let write = build_write(&colored);

    assert!(
        !write.body.as_object().unwrap().contains_key("colorId"),
        "event colors are read-only: {:#?}",
        write.body
    );
}

#[test]
fn new_exception_sends_series_reference_and_original_slot() {
    let mut exception = event();
    exception.recurring_event_id = Some(EventId("master-1".into()));
    exception.original_start_time = Some("2026-09-02T09:00:00Z".into());

    let write = build_write(&exception);

    assert_eq!(write.method, Method::POST);
    assert_eq!(body_field(&write, "recurringEventId"), "master-1");
    assert_eq!(
        body_field(&write, "originalStartTime"),
        &json!({ "dateTime": "2026-09-02T09:00:00Z" })
    );
}

#[test]
fn patching_an_exception_does_not_resend_immutable_series_fields() {
    let mut exception = event();
    exception.remote_id = Some(RemoteId("inst-1".into()));
    exception.recurring_event_id = Some(EventId("master-1".into()));
    exception.original_start_time = Some("2026-09-02T09:00:00Z".into());

    let write = build_write(&exception);

    assert_eq!(write.method, Method::PATCH);
    let body = write.body.as_object().unwrap();
    assert!(!body.contains_key("recurringEventId"));
    assert!(!body.contains_key("originalStartTime"));
}
