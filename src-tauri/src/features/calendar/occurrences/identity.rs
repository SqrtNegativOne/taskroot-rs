use crate::features::calendar::domain::{
    resolve_wall_clock, AppEvent, CalendarDateTime, EventId, EventTiming, DTO_DATE,
};
use std::collections::{HashMap, HashSet};

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
            (&event.recurring_event_id, event.original_start_time)
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
            format!("{id}@{}", start_date.format(DTO_DATE))
        }
        EventTiming::Timed { start, .. } => format!("{id}@{}", start.to_rfc3339()),
    }
}

/// The occurrence's start in the canonical `EXDATE` / `RECURRENCE-ID` shape
/// (`ics::read`): a bare date for all-day, the wall clock in the event's zone
/// for timed. The frontend writes it back verbatim to exclude one occurrence.
pub(super) fn canonical_start(timing: &EventTiming) -> CalendarDateTime {
    match timing {
        EventTiming::AllDay { start_date, .. } => CalendarDateTime::Date(*start_date),
        EventTiming::Timed {
            start, timezone, ..
        } => CalendarDateTime::from_utc_in_zone(*start, timezone.as_deref()),
    }
}

/// Canonicalize an override's `originalStartTime` to the same representation the
/// master occurrences use: a bare date for all-day, a UTC RFC 3339 instant for
/// timed. A timed `originalStartTime` is a wall clock in the exception's own
/// timezone (see `ics::read`), so it must be resolved in that zone.
pub(super) fn normalize_exception(original: CalendarDateTime, timezone: Option<&str>) -> String {
    match original {
        CalendarDateTime::Date(date) => date.format(DTO_DATE).to_string(),
        CalendarDateTime::WallClock(naive) => resolve_wall_clock(naive, timezone).to_rfc3339(),
        CalendarDateTime::Utc(instant) => instant.to_rfc3339(),
    }
}
