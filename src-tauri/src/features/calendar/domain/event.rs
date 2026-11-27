use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use taskroot_macros::Queryable;

use crate::features::calendar::caldav::model::CalendarComponent;

use super::{
    CalendarDateTime, Color, CollectionId, EventId, RemoteId, Tag, TaskId, TaskPriority,
};

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub enum EventScope {
    /// Only the selected occurrence; every other occurrence is untouched.
    This,
    /// The selected occurrence and every later one.
    Following,
    /// The whole series, including its exceptions.
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppCalendar {
    pub id: CollectionId,
    pub summary: String,
    #[specta(optional)]
    pub color: Option<Color>,
    /// Whether the current account may write to this collection.
    pub writable: bool,
    /// The component kind the collection stores.
    pub component: CalendarComponent,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Queryable)]
#[serde(rename_all = "camelCase")]
pub struct AppEvent {
    pub id: EventId,
    #[specta(optional)]
    pub remote_id: Option<RemoteId>,
    #[specta(optional)]
    #[query(label = "Calendar")]
    pub remote_collection_id: Option<CollectionId>,
    #[specta(optional)]
    pub task_id: Option<TaskId>,
    #[query(sortable)]
    pub title: String,
    #[specta(optional)]
    pub description: Option<String>,
    #[specta(optional)]
    #[query(sortable, filter_type = "number")]
    pub priority: Option<TaskPriority>,
    #[specta(optional)]
    #[query(filter_type = "relation:task_tags")]
    pub tags: Option<Vec<Tag>>,
    pub start_time: CalendarDateTime,
    pub end_time: CalendarDateTime,
    #[specta(optional)]
    pub rrule: Option<String>,
    #[specta(optional)]
    pub exdates: Option<Vec<CalendarDateTime>>,
    #[specta(optional)]
    pub recurring_event_id: Option<EventId>,
    #[specta(optional)]
    pub original_start_time: Option<CalendarDateTime>,
    #[specta(optional)]
    #[query(filter_type = "enum:EventStatus")]
    pub status: Option<EventStatus>,
    #[specta(optional)]
    pub updated_at: Option<DateTime<Utc>>,
    /// The `CREATED` timestamp (falling back to `DTSTAMP`), ISO 8601 UTC.
    #[specta(optional)]
    pub added: Option<DateTime<Utc>>,
    #[specta(optional)]
    pub color: Option<Color>,
    #[specta(optional)]
    pub etag: Option<String>,
    #[specta(optional)]
    pub is_all_day: Option<bool>,
    /// IANA timezone of a timed event's wall clock (`None` = UTC).
    #[specta(optional)]
    pub timezone: Option<String>,
}

impl AppEvent {
    /// Whether this row is a recurrence master or one of its exceptions.
    #[must_use]
    pub const fn is_recurring(&self) -> bool {
        self.rrule.is_some() || self.recurring_event_id.is_some()
    }

    /// Whether this event is all-day.
    ///
    /// An explicit `is_all_day: Some(true)` always wins; otherwise a floating
    /// date-only `start_time` implies all-day.
    #[must_use]
    pub const fn is_all_day(&self) -> bool {
        matches!(self.is_all_day, Some(true)) || self.start_time.is_date()
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
            priority: None,
            tags: None,
            start_time: "2026-08-12T10:00:00".parse().unwrap(),
            end_time: "2026-08-12T11:00:00".parse().unwrap(),
            rrule: None,
            exdates: None,
            recurring_event_id: None,
            original_start_time: None,
            status: Some(EventStatus::Confirmed),
            updated_at: Some("2026-08-26T07:31:42Z".parse().unwrap()),
            added: Some("2026-08-26T07:00:00Z".parse().unwrap()),
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
        assert!(json.contains(r#""updatedAt":"2026-08-26T07:31:42Z""#));

        let deserialized: AppEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, event.title);
        assert_eq!(deserialized.start_time, event.start_time);
        assert_eq!(deserialized.end_time, event.end_time);
        assert_eq!(deserialized.updated_at, event.updated_at);
        assert_eq!(deserialized.status, Some(EventStatus::Confirmed));
    }
}
