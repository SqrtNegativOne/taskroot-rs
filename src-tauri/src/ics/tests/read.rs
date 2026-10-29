//! Reading: resource bodies → DTOs.

use super::fixtures::*;
use crate::domain::{
    AppTaskStatus, EventId, EventStatus, GeoPoint, TaskAlarmAnchor, TaskAlarmTrigger, TaskId,
    TaskPriority,
};
use crate::ics::read_event_resource;

#[test]
fn mozilla_event_reads_every_owned_field() {
    let event = read_one(&fixture(MOZILLA));

    assert_eq!(event.id, EventId::from(MASTER_UID));
    assert_eq!(event.title, "CN");
    assert_eq!(event.start_time, "2026-10-19T11:00:00");
    assert_eq!(event.end_time, "2026-10-19T12:00:00");
    assert_eq!(event.timezone.as_deref(), Some("Asia/Kolkata"));
    assert_eq!(event.is_all_day, Some(false));
    assert_eq!(event.status, Some(EventStatus::Confirmed));
    assert_eq!(
        event.rrule.as_deref(),
        Some("RRULE:FREQ=WEEKLY;UNTIL=20261218T235959Z")
    );
    assert_eq!(event.exdates, Some(vec!["2026-10-19T11:00:00".to_string()]));
    assert_eq!(event.recurring_event_id, None);
    assert_eq!(event.original_start_time, None);
    assert_eq!(event.updated_at.as_deref(), Some("2026-10-19T06:49:00Z"));
    assert_eq!(event.color, None);
    assert_eq!(event.remote_id.as_ref().map(|id| id.0.as_str()), Some(HREF));
    assert_eq!(
        event.remote_collection_id.as_ref().map(|id| id.0.as_str()),
        Some(COLLECTION)
    );
}

#[test]
fn all_day_event_stays_a_floating_date() {
    let event = read_one(&fixture(ALL_DAY));

    assert_eq!(
        event.id,
        EventId::from("dsh-20261017-first-day-of-durga@dsh-import")
    );
    assert_eq!(event.title, "First Day of Durga");
    assert_eq!(event.start_time, "2026-10-17");
    assert_eq!(event.end_time, "2026-10-18");
    assert_eq!(event.is_all_day, Some(true));
    assert_eq!(event.timezone, None);
    assert_eq!(event.rrule, None);
    assert_eq!(event.exdates, None);
    assert_eq!(event.description, None);
}

#[test]
fn tasks_org_task_maps_the_workflow_and_the_priority_bucket() {
    let task = read_task(&fixture(TASK));

    assert_eq!(task.id, TaskId::from("1004415175551987562"));
    assert_eq!(task.title, "Brush");
    assert_eq!(task.status, Some(AppTaskStatus::Todo));
    assert_eq!(task.priority, Some(TaskPriority::Low));
    assert_eq!(task.added.as_deref(), Some("2026-10-23T12:13:08Z"));
    assert_eq!(task.notes, None);
    assert_eq!(task.due, None);
    assert_eq!(task.tags, None);
    assert_eq!(task.parent_task, None);
    assert_eq!(task.dependencies, None);
    assert_eq!(
        task.remote_collection_id.as_deref().map(String::as_str),
        Some(COLLECTION)
    );
    assert_eq!(task.remote_id.as_ref().map(|id| id.0.as_str()), Some(HREF));
}

#[test]
fn a_tasks_org_task_maps_start_due_recurrence_geo_and_reminder() {
    let task = read_task(&fixture(TASK_ALARM));

    assert_eq!(
        task.id,
        TaskId::from("1004415175551987563"),
        "the VTODO UID is the id"
    );
    assert_eq!(task.title, "Water the plants");
    assert_eq!(task.start.as_deref(), Some("2026-11-05T09:00:00"));
    assert_eq!(task.due.as_deref(), Some("2026-11-05T18:00:00"));
    assert_eq!(task.timezone.as_deref(), Some("Asia/Kolkata"));
    assert_eq!(task.rrule.as_deref(), Some("RRULE:FREQ=DAILY;COUNT=10"));
    assert_eq!(
        task.geo,
        Some(GeoPoint {
            latitude: 22.572_645,
            longitude: 88.363_892,
        })
    );
    assert_eq!(
        task.alarms,
        Some(vec![TaskAlarmTrigger::Relative {
            minutes_before: 15,
            anchor: TaskAlarmAnchor::End,
        }])
    );
    assert_eq!(task.priority, Some(TaskPriority::Medium));
    assert_eq!(task.parent_task, Some(TaskId::from("1004415175551987000")));
    assert_eq!(task.completed_at, None);
    assert_eq!(
        task.tags
            .map(|tags| tags.into_iter().map(|tag| tag.name).collect::<Vec<_>>()),
        Some(vec!["Garden".to_string(), "Home".to_string()])
    );
}

#[test]
fn an_exception_is_identified_by_uid_at_recurrence_id() {
    let raw = series_with_exception();
    let events = read_event_resource(&raw, HREF, COLLECTION).unwrap();

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].id, EventId::from(MASTER_UID));
    assert_eq!(
        events[1].id,
        EventId::from(format!("{MASTER_UID}@2026-10-26T11:00:00"))
    );
    assert_eq!(
        events[1].original_start_time.as_deref(),
        Some("2026-10-26T11:00:00")
    );
    assert_eq!(
        events[1]
            .recurring_event_id
            .as_ref()
            .map(|id| id.0.as_str()),
        Some(MASTER_UID)
    );
}
