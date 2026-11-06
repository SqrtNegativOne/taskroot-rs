use super::event_timing::{is_date_only, wall_clock_to_utc};
use super::{AppTaskStatus, CollectionId, TaskId};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Which of a task's two calendar dates a marker represents.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(
    export,
    export_to = "../../src/lib/bindings/TaskDateKind.generated.ts"
)]
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
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(
    export,
    export_to = "../../src/lib/bindings/TaskDateTiming.generated.ts"
)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TaskDateTiming {
    #[serde(rename_all = "camelCase")]
    AllDay {
        #[ts(type = "string")]
        date: NaiveDate,
    },
    #[serde(rename_all = "camelCase")]
    Timed {
        #[ts(type = "string")]
        start: DateTime<Utc>,
        timezone: Option<String>,
    },
}

impl TaskDateTiming {
    /// Normalize a task's canonical `start`/`due` value.
    ///
    /// Date-only values stay floating; timed wall clocks are resolved in the
    /// task's zone (falling back to UTC), matching the event projection.
    #[must_use]
    pub fn from_canonical(value: &str, timezone: Option<&str>) -> Option<Self> {
        if is_date_only(value) {
            return NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .ok()
                .map(|date| Self::AllDay { date });
        }
        wall_clock_to_utc(value, timezone).map(|start| Self::Timed {
            start,
            timezone: timezone.map(ToString::to_string),
        })
    }
}

/// One render-ready task date marker (`start` or `due`).
///
/// A task with both dates produces two markers, so the same task can appear
/// twice in the date grid.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(
    export,
    export_to = "../../src/lib/bindings/TaskDateInstance.generated.ts"
)]
#[serde(rename_all = "camelCase")]
pub struct TaskDateInstance {
    pub task_id: TaskId,
    pub kind: TaskDateKind,
    /// Stable identity for this marker: `<task-id>@<start|due>`.
    pub occurrence_key: String,
    pub title: String,
    pub timing: TaskDateTiming,
    #[ts(optional)]
    pub status: Option<AppTaskStatus>,
    #[ts(optional)]
    pub calendar_id: Option<CollectionId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_only_values_stay_floating() {
        assert_eq!(
            TaskDateTiming::from_canonical("2026-11-05", None),
            Some(TaskDateTiming::AllDay {
                date: NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
            })
        );
    }

    #[test]
    fn wall_clock_values_resolve_in_the_task_zone() {
        match TaskDateTiming::from_canonical("2026-10-19T11:00:00", Some("Asia/Kolkata")) {
            Some(TaskDateTiming::Timed { start, timezone }) => {
                assert_eq!(start.to_rfc3339(), "2026-10-19T05:30:00+00:00");
                assert_eq!(timezone.as_deref(), Some("Asia/Kolkata"));
            }
            other => panic!("expected timed timing, got {other:?}"),
        }
    }

    #[test]
    fn malformed_values_are_skipped() {
        assert!(TaskDateTiming::from_canonical("not-a-date", None).is_none());
    }
}
