use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use taskroot_macros::Queryable;
use ts_rs::TS;

use super::{CollectionId, RemoteId, TaskId};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(
    export,
    export_to = "../../src/lib/bindings/AppTaskStatus.generated.ts"
)]
#[serde(rename_all = "kebab-case")]
pub enum AppTaskStatus {
    Todo,
    NextUp,
    Doing,
    Done,
}

impl AppTaskStatus {
    #[must_use]
    pub fn all_values() -> Vec<String> {
        vec![
            "todo".into(),
            "next-up".into(),
            "doing".into(),
            "done".into(),
        ]
    }
}

#[derive(Debug, Clone, Serialize_repr, Deserialize_repr, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/lib/bindings/TaskPriority.generated.ts")]
#[ts(type = "0 | 1 | 2 | 3 | 4")]
#[repr(i32)]
pub enum TaskPriority {
    None = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    Urgent = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../src/lib/bindings/Tag.generated.ts")]
pub struct Tag {
    pub id: String,
    pub name: String,
    #[ts(optional)]
    pub color: Option<String>,
}

/// `GEO` (`RFC 5545` §3.8.1.6), the point tasks.org stores a geofence at.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq)]
#[ts(export, export_to = "../../src/lib/bindings/GeoPoint.generated.ts")]
#[serde(rename_all = "camelCase")]
pub struct GeoPoint {
    pub latitude: f64,
    pub longitude: f64,
}

/// What a relative reminder counts from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(
    export,
    export_to = "../../src/lib/bindings/TaskAlarmAnchor.generated.ts"
)]
#[serde(rename_all = "kebab-case")]
pub enum TaskAlarmAnchor {
    /// `TRIGGER;RELATED=START` (the default): the task's start, or its due time
    /// when it has no start.
    Start,
    /// `TRIGGER;RELATED=END`: the task's due time.
    End,
}

/// One `VALARM` trigger, in the two shapes tasks.org writes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(
    export,
    export_to = "../../src/lib/bindings/TaskAlarmTrigger.generated.ts"
)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum TaskAlarmTrigger {
    /// `TRIGGER;RELATED=…:-PT15M`. `minutesBefore` is positive for "before"
    /// and negative for "after"; `0` means "at the anchor".
    Relative {
        minutes_before: i32,
        anchor: TaskAlarmAnchor,
    },
    /// `TRIGGER;VALUE=DATE-TIME:…Z`, ISO 8601 UTC.
    Absolute { at: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Queryable)]
#[ts(export, export_to = "../../src/lib/bindings/AppTask.generated.ts")]
#[serde(rename_all = "camelCase")]
pub struct AppTask {
    #[query(sortable, label = "Created")]
    pub id: TaskId,
    #[query(sortable)]
    pub title: String,
    #[ts(optional)]
    #[query(filter_type = "enum:AppTaskStatus")]
    pub status: Option<AppTaskStatus>,
    #[ts(optional)]
    #[query(sortable, filter_type = "number")]
    pub priority: Option<TaskPriority>,
    #[ts(optional)]
    #[query(filter_type = "relation:task_tags")]
    pub tags: Option<Vec<Tag>>,
    #[ts(optional)]
    pub parent_task: Option<TaskId>,
    #[ts(optional)]
    pub dependencies: Option<Vec<TaskId>>,
    #[ts(optional)]
    pub est: Option<i32>,
    #[ts(optional)]
    pub added: Option<String>,
    #[ts(optional)]
    pub canvas_x: Option<f64>,
    #[ts(optional)]
    pub canvas_y: Option<f64>,
    #[ts(optional)]
    pub on_canvas: Option<bool>,
    #[ts(optional)]
    pub remote_id: Option<RemoteId>,
    /// Collection href this task lives in, matching [`AppEvent`](super::event::AppEvent).
    #[ts(optional)]
    #[query(label = "Collection")]
    pub remote_collection_id: Option<CollectionId>,
    #[ts(optional)]
    pub notes: Option<String>,
    #[ts(optional)]
    pub tabs: Option<String>,
    /// `DTSTART`: the task's start, which tasks.org also uses as "hide until".
    /// Canonical form matches [`Self::due`].
    #[ts(optional)]
    #[query(filter_type = "text")]
    pub start: Option<String>,
    #[ts(optional)]
    #[query(sortable, filter_type = "text")]
    pub due: Option<String>,
    /// The `COMPLETED` timestamp, ISO 8601 UTC.
    #[ts(optional)]
    pub completed_at: Option<String>,
    /// The raw `RRULE` line(s), wire form (`RRULE:FREQ=…`).
    #[ts(optional)]
    pub rrule: Option<String>,
    #[ts(optional)]
    pub geo: Option<GeoPoint>,
    /// `VALARM` triggers, in document order.
    #[ts(optional)]
    pub alarms: Option<Vec<TaskAlarmTrigger>>,
    /// IANA timezone `DTSTART`/`DUE` are written in; `None` writes them floating
    /// and `Some("UTC")` keeps a UTC-suffixed value's instant.
    #[ts(optional)]
    pub timezone: Option<String>,
    #[ts(optional)]
    pub updated_at: Option<String>,
    #[ts(optional)]
    pub etag: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_task_serialization() {
        let task = AppTask {
            id: "task-1".into(),
            title: "My Task".into(),
            status: Some(AppTaskStatus::NextUp),
            priority: None,
            tags: Some(vec![Tag {
                id: "tag-1".into(),
                name: "work".into(),
                color: None,
            }]),
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
        };

        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains(r#""status":"next-up""#));
        assert!(json.contains(r#""id":"task-1""#));

        let deserialized: AppTask = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status, Some(AppTaskStatus::NextUp));
    }
}
