//! Field-level patches for tasks and events.
//!
//! Sending only the fields that changed lets the backend merge edits onto a
//! freshly refetched resource on `412 Conflict`, preserving concurrent edits to
//! other fields from other `CalDAV` clients.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::{
    AppEvent, AppTask, AppTaskStatus, CalendarDateTime, CollectionId, Color, EventStatus, GeoPoint,
    Tag, TaskAlarmTrigger, TaskId, TaskPriority,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Type)]
#[serde(tag = "field", content = "value", rename_all = "camelCase")]
pub enum TaskPatch {
    Title(String),
    Status(Option<AppTaskStatus>),
    Priority(Option<TaskPriority>),
    Tags(Option<Vec<Tag>>),
    ParentTask(Option<TaskId>),
    Dependencies(Option<Vec<TaskId>>),
    Est(Option<i32>),
    TimeSpent(Option<i32>),
    RemoteCollectionId(Option<CollectionId>),
    Notes(Option<String>),
    Start(Option<CalendarDateTime>),
    Due(Option<CalendarDateTime>),
    CompletedAt(Option<DateTime<Utc>>),
    Rrule(Option<String>),
    Geo(Option<GeoPoint>),
    Alarms(Option<Vec<TaskAlarmTrigger>>),
    Timezone(Option<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(tag = "field", content = "value", rename_all = "camelCase")]
pub enum EventPatch {
    Title(String),
    Description(Option<String>),
    Priority(Option<TaskPriority>),
    Tags(Option<Vec<Tag>>),
    StartTime(CalendarDateTime),
    EndTime(CalendarDateTime),
    Rrule(Option<String>),
    Exdates(Option<Vec<CalendarDateTime>>),
    Status(Option<EventStatus>),
    Color(Option<Color>),
    IsAllDay(Option<bool>),
    Timezone(Option<String>),
    TaskId(Option<TaskId>),
    RemoteCollectionId(Option<CollectionId>),
}

/// Apply `patches` in order onto `task`.
///
/// Transitioning `status` away from [`AppTaskStatus::Done`] clears
/// `completed_at` unless `patches` also sets `CompletedAt` explicitly.
pub fn apply_task_patches(task: &mut AppTask, patches: &[TaskPatch]) {
    let has_completed_at = patches
        .iter()
        .any(|patch| matches!(patch, TaskPatch::CompletedAt(_)));

    for patch in patches {
        match patch {
            TaskPatch::Title(title) => task.title.clone_from(title),
            TaskPatch::Status(status) => {
                task.status.clone_from(status);
                if !has_completed_at && *status != Some(AppTaskStatus::Done) {
                    task.completed_at = None;
                }
            }
            TaskPatch::Priority(priority) => task.priority.clone_from(priority),
            TaskPatch::Tags(tags) => task.tags.clone_from(tags),
            TaskPatch::ParentTask(parent) => task.parent_task.clone_from(parent),
            TaskPatch::Dependencies(deps) => task.dependencies.clone_from(deps),
            TaskPatch::Est(est) => task.est = *est,
            TaskPatch::TimeSpent(time_spent) => task.time_spent = *time_spent,
            TaskPatch::RemoteCollectionId(col) => task.remote_collection_id.clone_from(col),
            TaskPatch::Notes(notes) => task.notes.clone_from(notes),
            TaskPatch::Start(start) => task.start = *start,
            TaskPatch::Due(due) => task.due = *due,
            TaskPatch::CompletedAt(completed_at) => task.completed_at = *completed_at,
            TaskPatch::Rrule(rrule) => task.rrule.clone_from(rrule),
            TaskPatch::Geo(geo) => task.geo = *geo,
            TaskPatch::Alarms(alarms) => task.alarms.clone_from(alarms),
            TaskPatch::Timezone(tz) => task.timezone.clone_from(tz),
        }
    }
}

/// Apply `patches` in order onto `event`.
pub fn apply_event_patches(event: &mut AppEvent, patches: &[EventPatch]) {
    for patch in patches {
        match patch {
            EventPatch::Title(title) => event.title.clone_from(title),
            EventPatch::Description(description) => event.description.clone_from(description),
            EventPatch::Priority(priority) => event.priority.clone_from(priority),
            EventPatch::Tags(tags) => event.tags.clone_from(tags),
            EventPatch::StartTime(start) => event.start_time = *start,
            EventPatch::EndTime(end) => event.end_time = *end,
            EventPatch::Rrule(rrule) => event.rrule.clone_from(rrule),
            EventPatch::Exdates(exdates) => event.exdates.clone_from(exdates),
            EventPatch::Status(status) => event.status.clone_from(status),
            EventPatch::Color(color) => event.color.clone_from(color),
            EventPatch::IsAllDay(is_all_day) => event.is_all_day = *is_all_day,
            EventPatch::Timezone(tz) => event.timezone.clone_from(tz),
            EventPatch::TaskId(task_id) => event.task_id.clone_from(task_id),
            EventPatch::RemoteCollectionId(col) => event.remote_collection_id.clone_from(col),
        }
    }
}

/// Whether the final `Status` entry in `patches` transitions the task to `Done`.
#[must_use]
pub fn patches_mark_task_done(patches: &[TaskPatch]) -> bool {
    patches
        .iter()
        .rev()
        .find_map(|patch| match patch {
            TaskPatch::Status(status) => Some(status),
            _ => None,
        })
        .is_some_and(|status| *status == Some(AppTaskStatus::Done))
}
