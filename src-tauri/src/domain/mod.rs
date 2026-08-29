use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub mod filters;
pub mod sigil;

pub use filters::*;
pub use sigil::*;

// We implement TS manually for string newtypes instead of using derive(TS)
// because ts-rs eager-parses serde attributes and warns about transparent/try_from,
// which causes console spam. This keeps the backend types safe and warnings clean!
macro_rules! impl_ts_string_newtype {
    ($($name:ident),+ $(,)?) => {
        $(
            impl ts_rs::TS for $name {
                type WithoutGenerics = Self;
                type OptionInnerType = Self;

                fn name(_: &ts_rs::Config) -> String { stringify!($name).to_string() }
                fn decl(cfg: &ts_rs::Config) -> String { format!("export type {} = string;", Self::name(cfg)) }
                fn decl_concrete(cfg: &ts_rs::Config) -> String { Self::decl(cfg) }
                fn inline(_: &ts_rs::Config) -> String { "string".to_string() }
                fn visit_dependencies(_: &mut impl ts_rs::TypeVisitor) {}
            }
            
            impl std::ops::Deref for $name {
                type Target = String;
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }
        )+

        #[cfg(test)]
        #[test]
        fn export_bindings_newtypes() {
            let config = ts_rs::Config::default();
            $(
                let decl = <$name as ts_rs::TS>::decl(&config);
                let path = format!("../../src/lib/bindings/{}.generated.ts", stringify!($name));
                std::fs::write(&path, format!("{decl}\n")).unwrap();
            )+
        }
    };
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, std::hash::Hash, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct TaskId(pub String);
impl std::fmt::Display for TaskId { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) } }
impl From<String> for TaskId { fn from(s: String) -> Self { Self(s) } }
impl From<&str> for TaskId { fn from(s: &str) -> Self { Self(s.to_string()) } }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, std::hash::Hash, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct EventId(pub String);
impl std::fmt::Display for EventId { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) } }
impl From<String> for EventId { fn from(s: String) -> Self { Self(s) } }
impl From<&str> for EventId { fn from(s: &str) -> Self { Self(s.to_string()) } }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, std::hash::Hash, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct RemoteId(pub String);
impl std::fmt::Display for RemoteId { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) } }
impl From<String> for RemoteId { fn from(s: String) -> Self { Self(s) } }
impl From<&str> for RemoteId { fn from(s: &str) -> Self { Self(s.to_string()) } }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, std::hash::Hash, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct CollectionId(pub String);
impl std::fmt::Display for CollectionId { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) } }
impl From<String> for CollectionId { fn from(s: String) -> Self { Self(s) } }
impl From<&str> for CollectionId { fn from(s: &str) -> Self { Self(s.to_string()) } }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, std::hash::Hash, sqlx::Type)]
#[serde(try_from = "String", into = "String")]
#[sqlx(transparent)]
pub struct Color(pub String);
impl TryFrom<String> for Color {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.starts_with('#') && (s.len() == 7 || s.len() == 9) { Ok(Self(s)) } else { Err(format!("Invalid color format: '{s}'")) }
    }
}
impl From<Color> for String { fn from(c: Color) -> Self { c.0 } }
impl std::fmt::Display for Color { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) } }

impl_ts_string_newtype!(TaskId, EventId, RemoteId, CollectionId, Color);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS, sqlx::Type)]
#[ts(export, export_to = "../../src/lib/bindings/WindowLabel.generated.ts")]
#[serde(rename_all = "kebab-case")]
#[sqlx(type_name = "TEXT", rename_all = "kebab-case")]
pub enum WindowLabel {
    Main,
    Launcher,
    Minitracker,
}

impl WindowLabel {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Launcher => "launcher",
            Self::Minitracker => "minitracker",
        }
    }
}


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

impl AppTaskStatus {
    #[must_use]
    pub fn all_values() -> Vec<String> {
        vec!["todo".into(), "next-up".into(), "doing".into(), "done".into()]
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../src/lib/bindings/ChecklistItem.generated.ts")]
pub struct ChecklistItem {
    #[serde(default = "default_checklist_id")]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub done: bool,
    #[serde(flatten)]
    #[ts(skip)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

fn default_checklist_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, sqlx::FromRow, PartialEq, Eq)]
#[ts(export, export_to = "../../src/lib/bindings/Tag.generated.ts")]
pub struct Tag {
    pub id: String,
    pub name: String,
    #[ts(optional)]
    pub color: Option<String>,
}

use taskroot_macros::Queryable;

#[derive(Debug, Clone, Serialize, Deserialize, TS, sqlx::FromRow, Queryable)]
#[ts(export, export_to = "../../src/lib/bindings/AppTask.generated.ts")]
#[serde(rename_all = "camelCase")]
pub struct AppTask {
    #[query(sortable, db_col = "t.id", label = "Created")]
    pub id: TaskId,
    #[query(sortable, db_col = "LOWER(t.title)")]
    pub title: String,
    #[ts(optional)]
    #[query(db_col = "t.status", filter_type = "enum:AppTaskStatus")]
    pub status: Option<AppTaskStatus>,
    #[ts(optional)]
    #[query(sortable, db_col = "COALESCE(t.priority, 0)", filter_type = "number")]
    pub priority: Option<TaskPriority>,
    #[ts(optional)]
    #[sqlx(json)]
    #[query(db_col = "", filter_type = "relation:task_tags")]
    pub tags: Option<Vec<Tag>>,
    #[ts(optional)]
    #[sqlx(json)]
    pub checklist: Option<Vec<ChecklistItem>>,
    #[ts(optional)]
    pub parent_task: Option<TaskId>,
    #[ts(optional)]
    #[sqlx(json)]
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
    #[ts(optional)]
    pub notes: Option<String>,
    #[ts(optional)]
    pub tabs: Option<String>,
    #[ts(optional)]
    #[query(sortable, db_col = "COALESCE(t.due, '9999')", filter_type = "text")]
    pub due: Option<String>,
    #[ts(optional)]
    pub updated_at: Option<String>,
    #[ts(optional)]
    pub etag: Option<String>,
    #[ts(optional)]
    pub dirty: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS, sqlx::Type)]
#[ts(export, export_to = "../../src/lib/bindings/EventStatus.generated.ts")]
#[serde(rename_all = "camelCase")]
#[sqlx(type_name = "TEXT", rename_all = "camelCase")]
pub enum EventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

impl EventStatus {
    #[must_use]
    pub fn all_values() -> Vec<String> {
        vec!["confirmed".into(), "tentative".into(), "cancelled".into()]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, sqlx::FromRow, Queryable)]
#[ts(export, export_to = "../../src/lib/bindings/AppEvent.generated.ts")]
#[serde(rename_all = "camelCase")]
pub struct AppEvent {
    pub id: EventId,
    #[ts(optional)]
    pub remote_id: Option<RemoteId>,
    #[ts(optional)]
    #[query(db_col = "remote_collection_id", label = "Calendar")]
    pub remote_collection_id: Option<CollectionId>,
    #[ts(optional)]
    pub task_id: Option<TaskId>,
    #[query(db_col = "LOWER(title)")]
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
    pub recurring_event_id: Option<EventId>,
    #[ts(optional)]
    pub original_start_time: Option<String>,
    #[ts(optional)]
    #[query(db_col = "status", filter_type = "enum:EventStatus")]
    pub status: Option<EventStatus>,
    #[ts(optional)]
    pub updated_at: Option<String>,
    #[ts(optional)]
    #[sqlx(default)]
    pub color: Option<Color>,
    #[ts(optional)]
    pub etag: Option<String>,
    #[ts(optional)]
    pub dirty: Option<bool>,
    #[ts(optional)]
    pub is_all_day: Option<bool>,
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
            checklist: None,
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
            status: Some(EventStatus::Confirmed),
            updated_at: Some("2026-08-26T07:31:42Z".into()),
            color: None,
            etag: None,
            dirty: None,
            is_all_day: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""id":"event-1""#));
        assert!(json.contains(r#""startTime":"2026-08-12T10:00:00""#));
        assert!(json.contains(r#""endTime":"2026-08-12T11:00:00""#));
        assert!(json.contains(r#""status":"confirmed""#));

        let deserialized: AppEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, event.title);
        assert_eq!(deserialized.start_time, event.start_time);
        assert_eq!(deserialized.end_time, event.end_time);
        assert_eq!(deserialized.status, Some(EventStatus::Confirmed));
    }
}
