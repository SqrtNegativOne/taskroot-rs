use crate::features::calendar::domain::{wall_clock_to_utc, AppEvent, EventId, EventTiming};
use chrono_tz::Tz as ChronoTz;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

pub(super) fn collect_series_rules(events: &[AppEvent]) -> HashMap<EventId, String> {
    events
        .iter()
        .filter_map(|event| Some((event.id.clone(), event.rrule.clone()?)))
        .collect()
}

pub(super) fn collect_exception_keys(events: &[AppEvent]) -> HashSet<String> {
    let mut exceptions = HashSet::new();
    for event in events {
        let (Some(master), Some(original)) =
            (&event.recurring_event_id, &event.original_start_time)
        else {
            continue;
        };
        exceptions.insert(format!(
            "{master}@{}",
            normalize_exception(original, event.timezone.as_deref())
        ));
    }
    exceptions
}

pub(super) fn occurrence_key(id: &EventId, timing: &EventTiming) -> String {
    match timing {
        EventTiming::AllDay { start_date, .. } => {
            format!("{id}@{}", start_date.format("%Y-%m-%d"))
        }
        EventTiming::Timed { start, .. } => format!("{id}@{}", start.to_rfc3339()),
    }
}

/// The occurrence's start in the canonical `EXDATE` / `RECURRENCE-ID` shape
/// (`ics::read`): a bare date for all-day, the wall clock in the event's zone
/// for timed. The frontend writes it back verbatim to exclude one occurrence.
pub(super) fn canonical_start(timing: &EventTiming) -> String {
    match timing {
        EventTiming::AllDay { start_date, .. } => start_date.format("%Y-%m-%d").to_string(),
        EventTiming::Timed {
            start, timezone, ..
        } => {
            let wall = timezone
                .as_deref()
                .and_then(|name| ChronoTz::from_str(name).ok())
                .map_or_else(
                    || start.naive_utc(),
                    |zone| start.with_timezone(&zone).naive_local(),
                );
            wall.format("%Y-%m-%dT%H:%M:%S").to_string()
        }
    }
}

/// Canonicalize an override's `originalStartTime` to the same representation the
/// master occurrences use: a bare date for all-day, a UTC RFC 3339 instant for
/// timed. A timed `originalStartTime` is a wall clock in the exception's own
/// timezone (see `ics::read`), so it must be resolved in that zone.
pub(super) fn normalize_exception(original: &str, timezone: Option<&str>) -> String {
    if original.len() == 10 && !original.contains('T') {
        return original.to_string();
    }
    wall_clock_to_utc(original, timezone)
        .map_or_else(|| original.to_string(), |instant| instant.to_rfc3339())
}
