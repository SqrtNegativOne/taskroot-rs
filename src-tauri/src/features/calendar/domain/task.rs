use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use specta::Type;
use taskroot_macros::Queryable;

use super::{CollectionId, RemoteId, TaskId};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "kebab-case")]
pub enum AppTaskStatus {
    Todo,
    NextUp,
    Done,
    Cancelled,
}

impl AppTaskStatus {
    #[must_use]
    pub fn all_values() -> Vec<String> {
        vec![
            "todo".into(),
            "next-up".into(),
            "done".into(),
            "cancelled".into(),
        ]
    }
}

#[derive(Debug, Clone, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(i32)]
pub enum TaskPriority {
    None = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    Urgent = 4,
}

mod priority_wire {
    use specta::Type;

    pub(super) struct Literal;

    impl Type for Literal {
        fn definition(_types: &mut specta::Types) -> specta::datatype::DataType {
            specta::datatype::DataType::Reference(specta_typescript::define("0 | 1 | 2 | 3 | 4"))
        }
    }

    #[derive(Type)]
    #[specta(transparent)]
    pub(super) struct TaskPriority(#[allow(dead_code)] Literal);
}

impl Type for TaskPriority {
    fn definition(types: &mut specta::Types) -> specta::datatype::DataType {
        priority_wire::TaskPriority::definition(types)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct Tag {
    pub id: String,
    pub name: String,
    #[specta(optional)]
    pub color: Option<String>,
}

/// `GEO` (`RFC 5545` §3.8.1.6), the point tasks.org stores a geofence at.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GeoPoint {
    pub latitude: f64,
    pub longitude: f64,
}

/// What a relative reminder counts from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "kebab-case")]
pub enum TaskAlarmAnchor {
    /// `TRIGGER;RELATED=START` (the default): the task's start, or its due time
    /// when it has no start.
    Start,
    /// `TRIGGER;RELATED=END`: the task's due time.
    End,
}

/// One `VALARM` trigger, in the two shapes tasks.org writes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Type, Queryable)]
#[serde(rename_all = "camelCase")]
pub struct AppTask {
    #[query(label = "ID")]
    pub id: TaskId,
    #[query(sortable)]
    pub title: String,
    #[specta(optional)]
    #[query(filter_type = "enum:AppTaskStatus")]
    pub status: Option<AppTaskStatus>,
    #[specta(optional)]
    #[query(sortable, filter_type = "number")]
    pub priority: Option<TaskPriority>,
    #[specta(optional)]
    #[query(filter_type = "relation:task_tags")]
    pub tags: Option<Vec<Tag>>,
    #[specta(optional)]
    pub parent_task: Option<TaskId>,
    #[specta(optional)]
    pub dependencies: Option<Vec<TaskId>>,
    #[specta(optional)]
    pub est: Option<i32>,
    #[specta(optional)]
    #[query(sortable, filter_type = "date", label = "Created")]
    pub added: Option<String>,
    #[specta(optional)]
    pub remote_id: Option<RemoteId>,
    /// Collection href this task lives in, matching `AppEvent`.
    #[specta(optional)]
    #[query(label = "Collection")]
    pub remote_collection_id: Option<CollectionId>,
    #[specta(optional)]
    pub notes: Option<String>,
    /// `DTSTART`: the task's start, which tasks.org also uses as "hide until".
    /// Canonical form matches [`Self::due`].
    #[specta(optional)]
    #[query(filter_type = "date")]
    pub start: Option<String>,
    #[specta(optional)]
    #[query(sortable, filter_type = "date")]
    pub due: Option<String>,
    /// The `COMPLETED` timestamp, ISO 8601 UTC.
    #[specta(optional)]
    pub completed_at: Option<String>,
    /// The raw `RRULE` line(s), wire form (`RRULE:FREQ=…`).
    #[specta(optional)]
    pub rrule: Option<String>,
    #[specta(optional)]
    pub geo: Option<GeoPoint>,
    /// `VALARM` triggers, in document order.
    #[specta(optional)]
    pub alarms: Option<Vec<TaskAlarmTrigger>>,
    /// IANA timezone `DTSTART`/`DUE` are written in; `None` writes them floating
    /// and `Some("UTC")` keeps a UTC-suffixed value's instant.
    #[specta(optional)]
    pub timezone: Option<String>,
    #[specta(optional)]
    pub updated_at: Option<String>,
    #[specta(optional)]
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
        };

        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains(r#""status":"next-up""#));
        assert!(json.contains(r#""id":"task-1""#));

        let deserialized: AppTask = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status, Some(AppTaskStatus::NextUp));
    }
}
