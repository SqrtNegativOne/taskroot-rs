use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub mod filters;
pub mod sigil;

pub use filters::*;
pub use sigil::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS, sqlx::Type)]
#[ts(
    export,
    export_to = "../../src/lib/bindings/AppTaskStatus.generated.ts"
)]
#[serde(rename_all = "kebab-case")]
#[sqlx(type_name = "TEXT", rename_all = "kebab-case")]
pub enum AppTaskStatus {
    Todo,
    NextUp,
    Doing,
    Done,
}

use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Clone, Serialize_repr, Deserialize_repr, PartialEq, Eq, TS, sqlx::Type)]
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/Subtask.generated.ts")]
pub struct Subtask {
    pub done: bool,
    #[serde(flatten)]
    #[ts(skip)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, sqlx::FromRow, PartialEq, Eq)]
#[ts(export, export_to = "../../src/lib/bindings/Tag.generated.ts")]
pub struct Tag {
    pub id: String,
    pub name: String,
    #[ts(optional)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../src/lib/bindings/AppTask.generated.ts")]
#[serde(rename_all = "camelCase")]
pub struct AppTask {
    pub id: String,
    pub title: String,
    #[ts(optional)]
    pub status: Option<AppTaskStatus>,
    #[ts(optional)]
    pub priority: Option<TaskPriority>,
    #[ts(optional)]
    #[sqlx(json)]
    pub tags: Option<Vec<Tag>>,
    #[ts(optional)]
    #[sqlx(json)]
    pub subtasks: Option<Vec<Subtask>>,
    #[ts(optional)]
    pub parent_task: Option<String>,
    #[ts(optional)]
    #[sqlx(json)]
    pub dependencies: Option<Vec<String>>,
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
    pub remote_id: Option<String>,
    #[ts(optional)]
    pub notes: Option<String>,
    #[ts(optional)]
    pub tabs: Option<String>,
    #[ts(optional)]
    pub due: Option<String>,
    #[serde(rename = "_deleted")]
    #[ts(optional)]
    pub deleted: Option<bool>,
    #[ts(optional, type = "number")]
    pub updated_at: Option<i64>,
    #[ts(optional)]
    pub etag: Option<String>,
    #[ts(optional)]
    pub dirty: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../src/lib/bindings/AppEvent.generated.ts")]
#[serde(rename_all = "camelCase")]
pub struct AppEvent {
    pub id: String,
    #[ts(optional)]
    pub remote_id: Option<String>,
    #[ts(optional)]
    pub remote_collection_id: Option<String>,
    #[ts(optional)]
    pub task_id: Option<String>,
    pub title: String,
    #[ts(optional)]
    pub description: Option<String>,
    pub start_time: String,
    pub end_time: String,
    #[ts(optional)]
    pub rrule: Option<String>,
    #[ts(optional)]
    #[sqlx(json)]
    pub exdates: Option<Vec<String>>,
    #[ts(optional)]
    pub recurring_event_id: Option<String>,
    #[ts(optional)]
    pub original_start_time: Option<String>,
    #[ts(optional)]
    pub cancelled: Option<bool>,
    #[ts(optional, type = "number")]
    pub updated_at: Option<i64>,
    #[ts(optional)]
    #[sqlx(default)]
    pub color: Option<String>,
    #[serde(rename = "_deleted")]
    #[ts(optional)]
    pub deleted: Option<bool>,
    #[ts(optional)]
    pub etag: Option<String>,
    #[ts(optional)]
    pub dirty: Option<bool>,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
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
            subtasks: None,
            parent_task: None,
            dependencies: None,
            est: None,
            added: None,
            canvas_x: None,
            canvas_y: None,
            on_canvas: None,
            remote_id: None,
            notes: None,
            tabs: None,
            due: None,
            deleted: None,
            updated_at: None,
            etag: None,
            dirty: None,
        };

        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains(r#""status":"next-up""#));
        assert!(json.contains(r#""id":"task-1""#));

        let deserialized: AppTask = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status, Some(AppTaskStatus::NextUp));
    }

    #[test]
    fn test_app_event_serialization() {
        let event = AppEvent {
            id: "event-1".into(),
            remote_id: None,
            remote_collection_id: None,
            task_id: None,
            title: "Meeting".into(),
            description: None,
            start_time: "2026-08-12T10:00:00".into(),
            end_time: "2026-08-12T11:00:00".into(),
            rrule: None,
            exdates: None,
            recurring_event_id: None,
            original_start_time: None,
            cancelled: None,
            updated_at: None,
            color: None,
            deleted: None,
            etag: None,
            dirty: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""id":"event-1""#));
        assert!(json.contains(r#""startTime":"2026-08-12T10:00:00""#));
        assert!(json.contains(r#""endTime":"2026-08-12T11:00:00""#));

        let deserialized: AppEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, event.title);
        assert_eq!(deserialized.start_time, event.start_time);
        assert_eq!(deserialized.end_time, event.end_time);
    }
}
