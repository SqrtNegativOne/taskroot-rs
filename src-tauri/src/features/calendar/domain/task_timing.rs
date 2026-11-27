use super::dates::{resolve_wall_clock, CalendarDateTime};
use super::{AppTaskStatus, CollectionId, TaskId};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

/// Which of a task's two calendar dates a marker represents.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub enum TaskDateKind {
    Start,
    Due,
}

impl TaskDateKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Due => "due",
        }
    }
}

/// A task date normalized for rendering: a floating day or a timed instant.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TaskDateTiming {
    #[serde(rename_all = "camelCase")]
    AllDay { date: NaiveDate },
    #[serde(rename_all = "camelCase")]
    Timed {
        start: DateTime<Utc>,
        timezone: Option<String>,
    },
}

impl TaskDateTiming {
    /// Normalize a task's typed `start`/`due` value.
    ///
    /// Date-only values stay floating; timed wall clocks are resolved in the
    /// task's zone (falling back to UTC), matching the event projection.
    #[must_use]
    pub fn from_calendar_date(value: CalendarDateTime, timezone: Option<&str>) -> Self {
        match value {
            CalendarDateTime::Date(date) => Self::AllDay { date },
            CalendarDateTime::WallClock(naive) => Self::Timed {
                start: resolve_wall_clock(naive, timezone),
                timezone: timezone.map(ToString::to_string),
            },
            CalendarDateTime::Utc(start) => Self::Timed {
                start,
                timezone: timezone.map(ToString::to_string),
            },
        }
    }
}

/// One render-ready task date marker (`start` or `due`).
///
/// A task with both dates produces two markers, so the same task can appear
/// twice in the date grid.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskDateInstance {
    pub task_id: TaskId,
    pub kind: TaskDateKind,
    /// Stable identity for this marker: `<task-id>@<start|due>`.
    pub occurrence_key: String,
    pub title: String,
    pub timing: TaskDateTiming,
    #[specta(optional)]
    pub status: Option<AppTaskStatus>,
    #[specta(optional)]
    pub calendar_id: Option<CollectionId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_only_values_stay_floating() {
        assert_eq!(
            TaskDateTiming::from_calendar_date("2026-11-05".parse().unwrap(), None),
            TaskDateTiming::AllDay {
                date: NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
            }
        );
    }

    #[test]
    fn wall_clock_values_resolve_in_the_task_zone() {
        match TaskDateTiming::from_calendar_date(
            "2026-10-19T11:00:00".parse().unwrap(),
            Some("Asia/Kolkata"),
        ) {
            TaskDateTiming::Timed { start, timezone } => {
                assert_eq!(start.to_rfc3339(), "2026-10-19T05:30:00+00:00");
                assert_eq!(timezone.as_deref(), Some("Asia/Kolkata"));
            }
            other @ TaskDateTiming::AllDay { .. } => {
                panic!("expected timed timing, got {other:?}")
            }
        }
    }
}
