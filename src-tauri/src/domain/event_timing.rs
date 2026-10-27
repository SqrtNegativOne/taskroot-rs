use super::{AppEvent, CollectionId, Color, EventId, EventStatus, TaskId};
use chrono::{DateTime, Days, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz as ChronoTz;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
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
        let start = wall_clock_to_utc(&event.start_time, event.timezone.as_deref())?;
        let end = wall_clock_to_utc(&event.end_time, event.timezone.as_deref())
            .filter(|end| *end > start)
            .unwrap_or(start);
        Some(Self::Timed {
            start,
            end,
            timezone: event.timezone.clone(),
        })
    }
}

/// Interpret a mirrored timed string as an instant.
///
/// A naive value (`YYYY-MM-DDTHH:MM:SS`, the ICS wall clock the reader produces) is
/// resolved in `timezone`, falling back to UTC when the zone is absent or unknown.
/// An RFC 3339 / trailing-`Z` value already carries its instant and is used as-is.
/// A local time that does not exist (a DST spring-forward gap) degrades to UTC
/// rather than dropping the occurrence.
#[must_use]
pub fn wall_clock_to_utc(value: &str, timezone: Option<&str>) -> Option<DateTime<Utc>> {
    if let Ok(datetime) = DateTime::parse_from_rfc3339(value) {
        return Some(datetime.with_timezone(&Utc));
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%SZ") {
        return Some(naive.and_utc());
    }
    let naive = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S"))
        .ok()?;
    let zone = timezone
        .and_then(|name| ChronoTz::from_str(name).ok())
        .unwrap_or(ChronoTz::UTC);
    match zone.from_local_datetime(&naive) {
        chrono::LocalResult::Single(datetime) => Some(datetime.with_timezone(&Utc)),
        chrono::LocalResult::Ambiguous(earliest, _) => Some(earliest.with_timezone(&Utc)),
        chrono::LocalResult::None => Some(naive.and_utc()),
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
        let timing = EventTiming::from_event(&event("2026-10-19T11:00:00", "2026-10-19T12:00:00"))
            .unwrap();
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
