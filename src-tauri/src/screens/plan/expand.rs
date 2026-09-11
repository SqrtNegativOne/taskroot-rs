//! The single boundary that turns mirrored [`AppEvent`] rows into
//! render-ready [`EventInstance`]s.
//!
//! Nothing outside this module interprets `start_time` / `end_time`. Callers
//! hand in raw rows plus a requested range and get back tagged
//! [`EventTiming`] values.

use crate::domain::{AppEvent, EventId, EventInstance, EventStatus, EventTiming};
use crate::time_utils::rrule_utils::{get_all_day_occurrences, get_occurrences};
use chrono::{DateTime, Days, NaiveDate, Utc};
use std::collections::HashSet;

/// Requested render window. Date-only boundaries are interpreted in the host's
/// local calendar (the user's timezone); instants are used as-is.
struct RenderRange {
    start_instant: DateTime<Utc>,
    end_instant: DateTime<Utc>,
    start_date: NaiveDate,
    end_date_inclusive: NaiveDate,
}

/// Expand mirrored event rows into instances overlapping the requested range.
#[must_use]
pub fn expand_event_instances(
    events: &[AppEvent],
    range_start_iso: &str,
    range_end_iso: &str,
) -> Vec<EventInstance> {
    let Some(range) = normalize_range(range_start_iso, range_end_iso) else {
        return Vec::new();
    };
    let exceptions = collect_exception_keys(events);

    let mut instances = Vec::new();
    for event in events {
        if event.status == Some(EventStatus::Cancelled) {
            continue;
        }
        let Some(timing) = EventTiming::from_event(event) else {
            continue;
        };

        if event.recurring_event_id.is_some() {
            push_override(event, timing, &range, &mut instances);
        } else if let Some(rrule) = &event.rrule {
            expand_recurring(event, rrule, timing, &range, &exceptions, &mut instances);
        } else if overlaps(&timing, &range) {
            let key = occurrence_key(&event.id, &timing);
            instances.push(build_instance(event, timing, key, false));
        }
    }

    instances
}

/// Project a single row without recurrence expansion (used for range-less queries).
#[must_use]
pub fn single_instance(event: &AppEvent) -> Option<EventInstance> {
    if event.status == Some(EventStatus::Cancelled) {
        return None;
    }
    let timing = EventTiming::from_event(event)?;
    let recurring = event.rrule.is_some() || event.recurring_event_id.is_some();
    let key = occurrence_key(&event.id, &timing);
    Some(build_instance(event, timing, key, recurring))
}

fn push_override(
    event: &AppEvent,
    timing: EventTiming,
    range: &RenderRange,
    instances: &mut Vec<EventInstance>,
) {
    if !overlaps(&timing, range) {
        return;
    }
    let master = event
        .recurring_event_id
        .clone()
        .unwrap_or_else(|| event.id.clone());
    let key = event.original_start_time.as_deref().map_or_else(
        || occurrence_key(&master, &timing),
        |original| format!("{}@{}", master, normalize_exception(original)),
    );
    instances.push(build_instance(event, timing, key, true));
}

fn expand_recurring(
    event: &AppEvent,
    rrule: &str,
    timing: EventTiming,
    range: &RenderRange,
    exceptions: &HashSet<String>,
    instances: &mut Vec<EventInstance>,
) {
    match timing {
        EventTiming::AllDay {
            start_date,
            end_date_exclusive,
        } => {
            let Ok(occurrences) = get_all_day_occurrences(
                rrule,
                start_date,
                event.timezone.as_deref(),
                range.start_date,
                range.end_date_inclusive,
            ) else {
                return;
            };
            let Some(span) = end_date_exclusive
                .signed_duration_since(start_date)
                .num_days()
                .try_into()
                .ok()
            else {
                return;
            };
            for date in occurrences {
                let key = format!("{}@{}", event.id, date.format("%Y-%m-%d"));
                if exceptions.contains(&key) {
                    continue;
                }
                let Some(end) = date.checked_add_days(Days::new(span)) else {
                    continue;
                };
                let occ = EventTiming::AllDay {
                    start_date: date,
                    end_date_exclusive: end,
                };
                if overlaps(&occ, range) {
                    instances.push(build_instance(event, occ, key, true));
                }
            }
        }
        EventTiming::Timed {
            start,
            end,
            timezone,
        } => {
            let Ok(occurrences) =
                get_occurrences(rrule, &start, &range.start_instant, &range.end_instant)
            else {
                return;
            };
            let duration = end.signed_duration_since(start);
            for occurrence in occurrences {
                let occ_start = occurrence.with_timezone(&Utc);
                let key = format!("{}@{}", event.id, occ_start.to_rfc3339());
                if exceptions.contains(&key) {
                    continue;
                }
                let occ_end = occ_start.checked_add_signed(duration).unwrap_or(occ_start);
                if occ_end > range.start_instant && occ_start < range.end_instant {
                    instances.push(build_instance(
                        event,
                        EventTiming::Timed {
                            start: occ_start,
                            end: occ_end,
                            timezone: timezone.clone(),
                        },
                        key,
                        true,
                    ));
                }
            }
        }
    }
}

fn build_instance(
    event: &AppEvent,
    timing: EventTiming,
    occurrence_key: String,
    recurring: bool,
) -> EventInstance {
    EventInstance {
        id: event.id.clone(),
        occurrence_key,
        title: event.title.clone(),
        description: event.description.clone(),
        timing,
        recurring,
        rrule: event.rrule.clone(),
        calendar_id: event.remote_collection_id.clone(),
        color: event.color.clone(),
        status: event.status.clone().unwrap_or(EventStatus::Confirmed),
        task_id: event.task_id.clone(),
    }
}

fn collect_exception_keys(events: &[AppEvent]) -> HashSet<String> {
    let mut exceptions = HashSet::new();
    for event in events {
        let (Some(master), Some(original)) =
            (&event.recurring_event_id, &event.original_start_time)
        else {
            continue;
        };
        exceptions.insert(format!("{master}@{}", normalize_exception(original)));
    }
    exceptions
}

fn occurrence_key(id: &EventId, timing: &EventTiming) -> String {
    match timing {
        EventTiming::AllDay { start_date, .. } => {
            format!("{id}@{}", start_date.format("%Y-%m-%d"))
        }
        EventTiming::Timed { start, .. } => format!("{id}@{}", start.to_rfc3339()),
    }
}

/// Canonicalize an override's `originalStartTime` to the same representation
/// the master occurrences use: bare date for all-day, UTC RFC 3339 for timed.
fn normalize_exception(original: &str) -> String {
    if let Ok(dt) = DateTime::parse_from_rfc3339(original) {
        return dt.with_timezone(&Utc).to_rfc3339();
    }
    original.to_string()
}

fn overlaps(timing: &EventTiming, range: &RenderRange) -> bool {
    match timing {
        EventTiming::AllDay {
            start_date,
            end_date_exclusive,
        } => *start_date <= range.end_date_inclusive && *end_date_exclusive > range.start_date,
        EventTiming::Timed { start, end, .. } => {
            *end > range.start_instant && *start < range.end_instant
        }
    }
}

fn normalize_range(range_start_iso: &str, range_end_iso: &str) -> Option<RenderRange> {
    let start_instant = parse_boundary_instant(range_start_iso, false)?;
    let end_instant = parse_boundary_instant(range_end_iso, true)?;
    let start_date = parse_boundary_date(range_start_iso)?;
    let end_date_inclusive = parse_boundary_date(range_end_iso)?;
    Some(RenderRange {
        start_instant,
        end_instant,
        start_date,
        end_date_inclusive,
    })
}

fn parse_boundary_date(iso: &str) -> Option<NaiveDate> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(iso) {
        return Some(dt.with_timezone(&chrono::Local).date_naive());
    }
    NaiveDate::parse_from_str(iso, "%Y-%m-%d").ok()
}

fn parse_boundary_instant(iso: &str, end_of_day: bool) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(iso) {
        return Some(dt.with_timezone(&Utc));
    }
    let date = NaiveDate::parse_from_str(iso, "%Y-%m-%d").ok()?;
    let time = if end_of_day {
        chrono::NaiveTime::from_hms_opt(23, 59, 59)?
    } else {
        chrono::NaiveTime::from_hms_opt(0, 0, 0)?
    };
    date.and_time(time)
        .and_local_timezone(chrono::Local)
        .single()
        .map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::domain::EventId;
    use chrono::Datelike;

    fn event(id: &str, start: &str, end: &str) -> AppEvent {
        AppEvent {
            id: EventId::from(id),
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

    fn timed_start(instance: &EventInstance) -> String {
        match &instance.timing {
            EventTiming::Timed { start, .. } => start.to_rfc3339(),
            EventTiming::AllDay { .. } => panic!("expected timed instance"),
        }
    }

    fn all_day_start(instance: &EventInstance) -> String {
        match &instance.timing {
            EventTiming::AllDay { start_date, .. } => start_date.format("%Y-%m-%d").to_string(),
            EventTiming::Timed { .. } => panic!("expected all-day instance"),
        }
    }

    #[test]
    fn daily_recurrence_far_from_dtstart_stays_visible() {
        let mut master = event("evt-1", "2025-01-01T09:00:00Z", "2025-01-01T09:30:00Z");
        master.rrule = Some("RRULE:FREQ=DAILY".into());

        let instances = expand_event_instances(
            &[master],
            "2026-09-01T00:00:00Z",
            "2026-09-03T23:59:59Z",
        );

        assert_eq!(instances.len(), 3);
        assert!(instances.iter().all(|i| i.recurring));
        assert!(instances
            .iter()
            .all(|i| i.id == EventId::from("evt-1")));
    }

    #[test]
    fn weekly_all_day_recurrence_renders_every_week() {
        let mut master = event("evt-1", "2026-09-07", "2026-09-08");
        master.rrule = Some("RRULE:FREQ=WEEKLY;BYDAY=MO".into());
        master.is_all_day = Some(true);

        let instances =
            expand_event_instances(&[master], "2026-09-07", "2026-10-04");

        let starts: Vec<String> = instances.iter().map(all_day_start).collect();
        assert_eq!(
            starts,
            vec!["2026-09-07", "2026-09-14", "2026-09-21", "2026-09-28"]
        );
    }

    #[test]
    fn weekly_all_day_recurrence_keeps_monday_in_negative_offset_timezone() {
        let mut master = event("evt-1", "2026-09-07", "2026-09-08");
        master.rrule = Some("RRULE:FREQ=WEEKLY;BYDAY=MO".into());
        master.is_all_day = Some(true);
        master.timezone = Some("America/Los_Angeles".into());

        let instances =
            expand_event_instances(&[master], "2026-09-07", "2026-10-04");

        assert_eq!(instances.len(), 4);
        assert!(instances.iter().all(|i| {
            NaiveDate::parse_from_str(&all_day_start(i), "%Y-%m-%d")
                .unwrap()
                .weekday()
                == chrono::Weekday::Mon
        }));
    }

    #[test]
    fn cancelled_all_day_occurrence_is_suppressed() {
        let mut master = event("evt-1", "2026-09-07", "2026-09-08");
        master.rrule = Some("RRULE:FREQ=WEEKLY;BYDAY=MO".into());
        master.is_all_day = Some(true);

        let mut cancelled = event("evt-2", "2026-09-14", "2026-09-15");
        cancelled.is_all_day = Some(true);
        cancelled.recurring_event_id = Some(EventId::from("evt-1"));
        cancelled.original_start_time = Some("2026-09-14".into());
        cancelled.status = Some(EventStatus::Cancelled);

        let instances =
            expand_event_instances(&[master, cancelled], "2026-09-07", "2026-10-04");

        let starts: Vec<String> = instances.iter().map(all_day_start).collect();
        assert_eq!(starts, vec!["2026-09-07", "2026-09-21", "2026-09-28"]);
    }

    #[test]
    fn exdate_inside_rrule_suppresses_all_day_occurrence() {
        let mut master = event("evt-1", "2026-09-07", "2026-09-08");
        master.rrule =
            Some("RRULE:FREQ=WEEKLY;BYDAY=MO\nEXDATE;VALUE=DATE:20260914".into());
        master.is_all_day = Some(true);

        let instances = expand_event_instances(&[master], "2026-09-07", "2026-10-04");

        let starts: Vec<String> = instances.iter().map(all_day_start).collect();
        assert_eq!(starts, vec!["2026-09-07", "2026-09-21", "2026-09-28"]);
    }

    #[test]
    fn exdate_inside_rrule_suppresses_timed_occurrence() {
        let mut master = event("evt-1", "2026-09-01T09:00:00Z", "2026-09-01T09:30:00Z");
        master.rrule = Some("RRULE:FREQ=DAILY;COUNT=3\nEXDATE:20260902T090000Z".into());

        let instances = expand_event_instances(
            &[master],
            "2026-09-01T00:00:00Z",
            "2026-09-03T23:59:59Z",
        );

        let mut starts: Vec<String> = instances.iter().map(timed_start).collect();
        starts.sort();
        assert_eq!(
            starts,
            vec!["2026-09-01T09:00:00+00:00", "2026-09-03T09:00:00+00:00"]
        );
    }

    #[test]
    fn moved_occurrence_renders_once_at_its_new_time() {
        let mut master = event("evt-1", "2026-09-01T09:00:00Z", "2026-09-01T09:30:00Z");
        master.rrule = Some("RRULE:FREQ=DAILY;COUNT=3".into());

        let mut moved = event("evt-2", "2026-09-02T14:00:00Z", "2026-09-02T14:30:00Z");
        moved.recurring_event_id = Some(EventId::from("evt-1"));
        moved.original_start_time = Some("2026-09-02T09:00:00Z".into());

        let instances = expand_event_instances(
            &[master, moved],
            "2026-09-01T00:00:00Z",
            "2026-09-03T23:59:59Z",
        );

        let mut starts: Vec<String> = instances.iter().map(timed_start).collect();
        starts.sort();
        assert_eq!(
            starts,
            vec![
                "2026-09-01T09:00:00+00:00",
                "2026-09-02T14:00:00+00:00",
                "2026-09-03T09:00:00+00:00",
            ],
            "the moved occurrence replaces the original slot exactly once"
        );
    }

    #[test]
    fn cancelled_standalone_event_is_not_rendered() {
        let mut cancelled = event("evt-1", "2026-09-02T09:00:00Z", "2026-09-02T09:30:00Z");
        cancelled.status = Some(EventStatus::Cancelled);

        let instances = expand_event_instances(
            &[cancelled],
            "2026-09-01T00:00:00Z",
            "2026-09-03T23:59:59Z",
        );

        assert!(instances.is_empty());
    }

    #[test]
    fn hourly_recurrence_produces_distinct_occurrence_keys() {
        let mut master = event("evt-1", "2026-09-02T08:00:00Z", "2026-09-02T08:30:00Z");
        master.rrule = Some("RRULE:FREQ=HOURLY;COUNT=4".into());

        let instances = expand_event_instances(
            &[master],
            "2026-09-02T00:00:00Z",
            "2026-09-02T23:59:59Z",
        );

        assert_eq!(instances.len(), 4);
        let keys: HashSet<&str> = instances.iter().map(|i| i.occurrence_key.as_str()).collect();
        assert_eq!(keys.len(), 4, "sub-daily instances need unique keys");
    }

    #[test]
    fn all_day_events_are_kept_only_when_they_overlap_the_range() {
        let inside = event("evt-1", "2026-09-02", "2026-09-03");
        let outside = event("evt-2", "2026-09-10", "2026-09-11");

        let instances = expand_event_instances(
            &[inside, outside],
            "2026-09-01T00:00:00Z",
            "2026-09-07T23:59:59Z",
        );

        assert_eq!(instances.len(), 1);
        assert_eq!(all_day_start(&instances[0]), "2026-09-02");
    }
}
