use super::{
    AppEvent, AppTaskStatus, CalendarDateTime, CollectionId, Color, EventId, EventStatus, TaskId,
};
use chrono::{DateTime, Days, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

/// The one place that normalizes a mirrored event into render-ready timing.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EventTiming {
    #[serde(rename_all = "camelCase")]
    AllDay {
        start_date: NaiveDate,
        end_date_exclusive: NaiveDate,
    },
    #[serde(rename_all = "camelCase")]
    Timed {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        timezone: Option<String>,
    },
}

impl EventTiming {
    /// Normalize a mirrored event row into render-ready timing.
    #[must_use]
    pub fn from_event(event: &AppEvent) -> Option<Self> {
        if event.is_all_day() {
            return Self::all_day_from_event(event);
        }
        Self::timed_from_event(event)
    }

    fn all_day_from_event(event: &AppEvent) -> Option<Self> {
        let start_date = event.start_time.date();
        let parsed_end = event.end_time.date();
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
        let start = event.start_time.to_utc(event.timezone.as_deref())?;
        let end = event
            .end_time
            .to_utc(event.timezone.as_deref())
            .filter(|end| *end > start)
            .unwrap_or(start);
        Some(Self::Timed {
            start,
            end,
            timezone: event.timezone.clone(),
        })
    }
}

/// A single render-ready occurrence produced by the projection boundary.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventInstance {
    /// Row id for this occurrence: the master for rule-expanded instances, or
    /// the override row for a moved instance. Edits target this id.
    pub id: EventId,
    /// Stable identity for one occurrence, e.g. `master@2026-09-07`.
    pub occurrence_key: String,
    /// The canonical value of this occurrence for `EXDATE` / `RECURRENCE-ID`:
    /// `YYYY-MM-DD` for all-day, wall clock in the event's zone for timed.
    /// Excluding it removes exactly this occurrence.
    pub occurrence_start: CalendarDateTime,
    pub title: String,
    #[specta(optional)]
    pub description: Option<String>,
    pub timing: EventTiming,
    /// True when this instance came from a recurrence rule.
    pub recurring: bool,
    /// The rule of the series this occurrence belongs to (the master's rule
    /// for a moved exception); `None` for a one-off event.
    #[specta(optional)]
    pub rrule: Option<String>,
    #[specta(optional)]
    pub calendar_id: Option<CollectionId>,
    #[specta(optional)]
    pub color: Option<Color>,
    pub status: EventStatus,
    #[specta(optional)]
    pub task_id: Option<TaskId>,
    #[specta(optional)]
    pub task_status: Option<AppTaskStatus>,
}

#[cfg(test)]
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
            priority: None,
            tags: None,
            start_time: start.parse().unwrap(),
            end_time: end.parse().unwrap(),
            rrule: None,
            exdates: None,
            recurring_event_id: None,
            original_start_time: None,
            status: Some(EventStatus::Confirmed),
            updated_at: None,
            added: None,
            color: None,
            etag: None,
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
        let timing = EventTiming::from_event(&event(
            "2026-09-07T09:00:00-07:00",
            "2026-09-07T10:00:00-07:00",
        ))
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

    #[test]
    fn naive_wall_clock_is_interpreted_in_the_event_timezone() {
        let mut row = event("2026-10-19T11:00:00", "2026-10-19T12:00:00");
        row.timezone = Some("Asia/Kolkata".into());

        match EventTiming::from_event(&row).unwrap() {
            EventTiming::Timed {
                start,
                end,
                timezone,
            } => {
                assert_eq!(start.to_rfc3339(), "2026-10-19T05:30:00+00:00");
                assert_eq!(end.to_rfc3339(), "2026-10-19T06:30:00+00:00");
                assert_eq!(timezone.as_deref(), Some("Asia/Kolkata"));
            }
            EventTiming::AllDay { .. } => panic!("expected timed timing"),
        }
    }

    #[test]
    fn naive_wall_clock_falls_back_to_utc_without_a_timezone() {
        let timing =
            EventTiming::from_event(&event("2026-10-19T11:00:00", "2026-10-19T12:00:00")).unwrap();
        match timing {
            EventTiming::Timed { start, .. } => {
                assert_eq!(start.to_rfc3339(), "2026-10-19T11:00:00+00:00");
            }
            EventTiming::AllDay { .. } => panic!("expected timed timing"),
        }
    }

    #[test]
    fn a_wall_clock_across_a_dst_transition_keeps_its_local_hour() {
        for (wall, expected) in [
            ("2026-03-23T09:00:00", "2026-03-23T09:00:00+00:00"),
            ("2026-03-30T09:00:00", "2026-03-30T08:00:00+00:00"),
        ] {
            let mut row = event(wall, wall);
            row.timezone = Some("Europe/London".into());
            match EventTiming::from_event(&row).unwrap() {
                EventTiming::Timed { start, .. } => assert_eq!(start.to_rfc3339(), expected),
                EventTiming::AllDay { .. } => panic!("expected timed timing"),
            }
        }
    }

    #[test]
    fn a_nonexistent_local_time_degrades_to_utc_instead_of_vanishing() {
        // 01:30 does not exist in Europe/London on 2026-03-29 (clocks go 01:00 →
        // 02:00), so the row must still produce an instant.
        let mut row = event("2026-03-29T01:30:00", "2026-03-29T02:30:00");
        row.timezone = Some("Europe/London".into());
        assert!(EventTiming::from_event(&row).is_some());
    }
}
