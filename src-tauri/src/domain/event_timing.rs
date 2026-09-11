use super::{AppEvent, CollectionId, Color, EventId, EventStatus, TaskId};
use chrono::{DateTime, Days, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

const YMD_LENGTH: usize = 10;

/// The one place that decides whether a mirrored event is all-day or timed.
///
/// Nothing outside this module may interpret `AppEvent::start_time` /
/// `end_time`; those remain wire-format strings for the push path only.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../src/lib/bindings/EventTiming.generated.ts")]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EventTiming {
    #[serde(rename_all = "camelCase")]
    AllDay {
        #[ts(type = "string")]
        start_date: NaiveDate,
        #[ts(type = "string")]
        end_date_exclusive: NaiveDate,
    },
    #[serde(rename_all = "camelCase")]
    Timed {
        #[ts(type = "string")]
        start: DateTime<Utc>,
        #[ts(type = "string")]
        end: DateTime<Utc>,
        timezone: Option<String>,
    },
}

impl EventTiming {
    /// Normalize a mirrored event row into render-ready timing.
    ///
    /// Returns `None` when the row carries no parseable start, in which case
    /// the render path drops it rather than guessing.
    #[must_use]
    pub fn from_event(event: &AppEvent) -> Option<Self> {
        if is_all_day(event) {
            return Self::all_day_from_event(event);
        }
        Self::timed_from_event(event)
    }

    /// Floating date if this timing is all-day.
    #[must_use]
    pub const fn start_date(&self) -> Option<NaiveDate> {
        match self {
            Self::AllDay { start_date, .. } => Some(*start_date),
            Self::Timed { .. } => None,
        }
    }

    fn all_day_from_event(event: &AppEvent) -> Option<Self> {
        let start_date = parse_date(&event.start_time)?;
        let parsed_end = parse_date(&event.end_time)?;
        let end_date_exclusive = if parsed_end > start_date {
            parsed_end
        } else {
            start_date.checked_add_days(Days::new(1))?
        };
        Some(Self::AllDay {
            start_date,
            end_date_exclusive,
        })
    }

    fn timed_from_event(event: &AppEvent) -> Option<Self> {
        let start = parse_instant(&event.start_time)?;
        let end = parse_instant(&event.end_time).filter(|end| *end > start).unwrap_or(start);
        Some(Self::Timed {
            start,
            end,
            timezone: event.timezone.clone(),
        })
    }
}

fn is_all_day(event: &AppEvent) -> bool {
    event.is_all_day.unwrap_or(false) || is_date_only(&event.start_time)
}

fn is_date_only(value: &str) -> bool {
    value.len() == YMD_LENGTH && !value.contains('T') && !value.contains(' ')
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    let trimmed = value.split(['T', ' ']).next().unwrap_or(value);
    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").ok()
}

fn parse_instant(value: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.with_timezone(&Utc));
    }
    let naive = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S"))
        .ok()?;
    Some(naive.and_utc())
}

/// A single render-ready occurrence produced by the projection boundary.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../src/lib/bindings/EventInstance.generated.ts")]
#[serde(rename_all = "camelCase")]
pub struct EventInstance {
    /// Row id for this occurrence: the master for rule-expanded instances, or
    /// the override row for a moved instance. Edits target this id.
    pub id: EventId,
    /// Stable identity for one occurrence, e.g. `master@2026-09-07`.
    pub occurrence_key: String,
    pub title: String,
    #[ts(optional)]
    pub description: Option<String>,
    pub timing: EventTiming,
    /// True when this instance came from a recurrence rule.
    pub recurring: bool,
    #[ts(optional)]
    pub rrule: Option<String>,
    #[ts(optional)]
    pub calendar_id: Option<CollectionId>,
    #[ts(optional)]
    pub color: Option<Color>,
    pub status: EventStatus,
    #[ts(optional)]
    pub task_id: Option<TaskId>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn event(start: &str, end: &str) -> AppEvent {
        AppEvent {
            id: "evt".into(),
            remote_id: None,
            remote_collection_id: None,
            task_id: None,
            title: "Event".into(),
            description: None,
            start_time: start.into(),
            end_time: end.into(),
            rrule: None,
            exdates: None,
            recurring_event_id: None,
            original_start_time: None,
            status: Some(EventStatus::Confirmed),
            updated_at: None,
            color: None,
            etag: None,
            dirty: None,
            is_all_day: None,
            timezone: None,
        }
    }

    #[test]
    fn date_only_rows_become_all_day_timing() {
        let timing = EventTiming::from_event(&event("2026-09-07", "2026-09-09")).unwrap();
        assert_eq!(
            timing,
            EventTiming::AllDay {
                start_date: NaiveDate::from_ymd_opt(2026, 9, 7).unwrap(),
                end_date_exclusive: NaiveDate::from_ymd_opt(2026, 9, 9).unwrap(),
            }
        );
    }

    #[test]
    fn single_day_all_day_defaults_to_one_day_span() {
        let timing = EventTiming::from_event(&event("2026-09-07", "2026-09-07")).unwrap();
        assert_eq!(
            timing,
            EventTiming::AllDay {
                start_date: NaiveDate::from_ymd_opt(2026, 9, 7).unwrap(),
                end_date_exclusive: NaiveDate::from_ymd_opt(2026, 9, 8).unwrap(),
            }
        );
    }

    #[test]
    fn rfc3339_rows_become_timed_timing_in_utc() {
        let timing =
            EventTiming::from_event(&event("2026-09-07T09:00:00-07:00", "2026-09-07T10:00:00-07:00"))
                .unwrap();
        match timing {
            EventTiming::Timed { start, end, .. } => {
                assert_eq!(start.to_rfc3339(), "2026-09-07T16:00:00+00:00");
                assert_eq!(end.to_rfc3339(), "2026-09-07T17:00:00+00:00");
            }
            EventTiming::AllDay { .. } => panic!("expected timed timing"),
        }
    }

    #[test]
    fn is_all_day_flag_wins_over_date_only_heuristic() {
        let mut row = event("2026-09-07T09:00:00Z", "2026-09-07T10:00:00Z");
        row.is_all_day = Some(true);
        let timing = EventTiming::from_event(&row).unwrap();
        assert!(matches!(timing, EventTiming::AllDay { .. }));
    }
}
