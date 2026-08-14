use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/lib/bindings/AppTaskStatus.ts")]
#[serde(rename_all = "kebab-case")]
pub enum AppTaskStatus {
    Todo,
    NextUp,
    Doing,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/Subtask.ts")]
pub struct Subtask {
    pub done: bool,
    #[serde(flatten)]
    #[ts(skip)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/AppTask.ts")]
#[serde(rename_all = "camelCase")]
pub struct AppTask {
    pub id: String,
    pub title: String,
    #[ts(optional)]
    pub status: Option<AppTaskStatus>,
    #[ts(optional)]
    pub priority: Option<i32>,
    #[ts(optional)]
    pub tags: Option<Vec<String>>,
    #[ts(optional)]
    pub subtasks: Option<Vec<Subtask>>,
    #[ts(optional)]
    pub parent_task: Option<String>,
    #[ts(optional)]
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
    #[ts(optional)]
    pub updated_at: Option<i64>,
    #[ts(optional)]
    pub etag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/lib/bindings/AppEventType.ts")]
#[serde(rename_all = "lowercase")]
pub enum AppEventType {
    Busy,
    Info,
    Plan,
    Log,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/AppEvent.ts")]
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
    #[serde(rename = "type")]
    pub event_type: AppEventType,
    #[ts(optional)]
    pub rrule: Option<String>,
    #[ts(optional)]
    pub exdates: Option<Vec<String>>,
    #[ts(optional)]
    pub recurring_event_id: Option<String>,
    #[ts(optional)]
    pub original_start_time: Option<String>,
    #[ts(optional)]
    pub cancelled: Option<bool>,
    #[ts(optional)]
    pub updated_at: Option<i64>,
    #[ts(optional)]
    pub color: Option<String>,
    #[serde(rename = "_deleted")]
    #[ts(optional)]
    pub deleted: Option<bool>,
    #[ts(optional)]
    pub etag: Option<String>,
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
            tags: Some(vec!["work".into()]),
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
            event_type: AppEventType::Busy,
            rrule: None,
            exdates: None,
            recurring_event_id: None,
            original_start_time: None,
            cancelled: None,
            updated_at: None,
            color: None,
            deleted: None,
            etag: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"busy""#));

        let deserialized: AppEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.event_type, AppEventType::Busy);
    }
}
