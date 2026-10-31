use serde::{Deserialize, Serialize};
use taskroot_macros::Queryable;
use ts_rs::TS;

use crate::features::calendar::caldav::model::CalendarComponent;

use super::{Color, CollectionId, EventId, RemoteId, TaskId};

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../src/lib/bindings/EventStatus.generated.ts")]
#[serde(rename_all = "camelCase")]
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

/// Which occurrences of a recurring event a write applies to.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/lib/bindings/EventScope.generated.ts")]
#[serde(rename_all = "camelCase")]
pub enum EventScope {
    /// Only the selected occurrence; every other occurrence is untouched.
    This,
    /// The selected occurrence and every later one.
    Following,
    /// The whole series, including its exceptions.
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../src/lib/bindings/AppCalendar.generated.ts")]
#[serde(rename_all = "camelCase")]
pub struct AppCalendar {
    pub id: CollectionId,
    pub summary: String,
    #[ts(optional)]
    pub color: Option<Color>,
    /// Whether the current account may write to this collection.
    pub writable: bool,
    /// The component kind the collection stores.
    pub component: CalendarComponent,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Queryable)]
#[ts(export, export_to = "../../src/lib/bindings/AppEvent.generated.ts")]
#[serde(rename_all = "camelCase")]
pub struct AppEvent {
    pub id: EventId,
    #[ts(optional)]
    pub remote_id: Option<RemoteId>,
    #[ts(optional)]
    #[query(label = "Calendar")]
    pub remote_collection_id: Option<CollectionId>,
    #[ts(optional)]
    pub task_id: Option<TaskId>,
    #[query(sortable)]
    pub title: String,
    #[ts(optional)]
    pub description: Option<String>,
    pub start_time: String,
    pub end_time: String,
    #[ts(optional)]
    pub rrule: Option<String>,
    #[ts(optional)]
    pub exdates: Option<Vec<String>>,
    #[ts(optional)]
    pub recurring_event_id: Option<EventId>,
    #[ts(optional)]
    pub original_start_time: Option<String>,
    #[ts(optional)]
    #[query(filter_type = "enum:EventStatus")]
    pub status: Option<EventStatus>,
    #[ts(optional)]
    pub updated_at: Option<String>,
    #[ts(optional)]
    pub color: Option<Color>,
    #[ts(optional)]
    pub etag: Option<String>,
    #[ts(optional)]
    pub is_all_day: Option<bool>,
    /// IANA timezone of a timed event's wall clock (`None` = UTC).
    #[ts(optional)]
    pub timezone: Option<String>,
}

impl AppEvent {
    /// Whether this row is a recurrence master or one of its exceptions.
    #[must_use]
    pub const fn is_recurring(&self) -> bool {
        self.rrule.is_some() || self.recurring_event_id.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            is_all_day: None,
            timezone: None,
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
