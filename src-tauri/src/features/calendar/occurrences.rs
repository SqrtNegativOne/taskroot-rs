//! Recurrence projection from stored [`AppEvent`] rows into render-ready
//! [`EventInstance`]s. [`EventTiming::from_event`] owns timestamp normalization;
//! this module preserves full exception/rule context separately from selection.

use crate::features::calendar::domain::timing::{get_all_day_occurrences, get_occurrences};
use crate::features::calendar::domain::{
    AppEvent, CalendarDateTime, EventId, EventInstance, EventStatus, EventTiming, DTO_DATE,
};
use crate::features::calendar::window::RenderWindow;
use chrono::Days;
use std::collections::{HashMap, HashSet};

mod identity;
use identity::{
    canonical_start, collect_exception_keys, collect_series_rules, normalize_exception,
    occurrence_key,
};

/// Expand mirrored event rows into instances overlapping the requested range.
#[must_use]
pub fn expand_event_instances(
    events: &[AppEvent],
    range_start_iso: &str,
    range_end_iso: &str,
) -> Vec<EventInstance> {
    expand_selected_event_instances(events, events, range_start_iso, range_end_iso)
}

pub(super) fn expand_selected_event_instances(
    context: &[AppEvent],
    selected: &[AppEvent],
    range_start_iso: &str,
    range_end_iso: &str,
) -> Vec<EventInstance> {
    let Some(range) = RenderWindow::from_iso(range_start_iso, range_end_iso) else {
        return Vec::new();
    };
    let exceptions = collect_exception_keys(context);
    let series_rules = collect_series_rules(context);

    let mut instances = Vec::new();
    for event in selected {
        if event.status == Some(EventStatus::Cancelled) {
            continue;
        }
        let Some(timing) = EventTiming::from_event(event) else {
            continue;
        };

        if event.recurring_event_id.is_some() {
            push_override(event, timing, &range, &series_rules, &mut instances);
        } else if let Some(rrule) = &event.rrule {
            expand_recurring(event, rrule, timing, &range, &exceptions, &mut instances);
        } else if overlaps(&timing, &range) {
            let key = occurrence_key(&event.id, &timing);
            let start = canonical_start(&timing);
            instances.push(build_instance(event, timing, key, start, false));
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
    let recurring = event.is_recurring();
    let key = occurrence_key(&event.id, &timing);
    let start = canonical_start(&timing);
    Some(build_instance(event, timing, key, start, recurring))
}

fn push_override(
    event: &AppEvent,
    timing: EventTiming,
    range: &RenderWindow,
    series_rules: &HashMap<EventId, String>,
    instances: &mut Vec<EventInstance>,
) {
    if !overlaps(&timing, range) {
        return;
    }
    let master = event
        .recurring_event_id
        .clone()
        .unwrap_or_else(|| event.id.clone());
    let key = event.original_start_time.map_or_else(
        || occurrence_key(&master, &timing),
        |original| {
            format!(
                "{}@{}",
                master,
                normalize_exception(original, event.timezone.as_deref())
            )
        },
    );
    let start = event
        .original_start_time
        .unwrap_or_else(|| canonical_start(&timing));
    let mut instance = build_instance(event, timing, key, start, true);
    if instance.rrule.is_none() {
        instance.rrule = series_rules.get(&master).cloned();
    }
    instances.push(instance);
}

fn expand_recurring(
    event: &AppEvent,
    rrule: &str,
    timing: EventTiming,
    range: &RenderWindow,
    exceptions: &HashSet<String>,
    instances: &mut Vec<EventInstance>,
) {
    match timing {
        EventTiming::AllDay {
            start_date,
            end_date_exclusive,
        } => {
            let Ok(span) = u64::try_from(end_date_exclusive.signed_duration_since(start_date).num_days()) else {
                return;
            };
            let search_start = range
                .start_date
                .checked_sub_days(Days::new(span.saturating_sub(1)))
                .unwrap_or(range.start_date);
            let Ok(occurrences) = get_all_day_occurrences(
                rrule,
                start_date,
                event.timezone.as_deref(),
                event.exdates.as_deref().unwrap_or_default(),
                search_start,
                range.end_date_inclusive,
            ) else {
                return;
            };
            for date in occurrences {
                let key = format!("{}@{}", event.id, date.format(DTO_DATE));
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
                    let start = canonical_start(&occ);
                    instances.push(build_instance(event, occ, key, start, true));
                }
            }
        }
        EventTiming::Timed {
            start,
            end,
            timezone,
        } => {
            let duration = end.signed_duration_since(start);
            let search_start = range
                .start_instant
                .checked_sub_signed(duration)
                .unwrap_or(range.start_instant);
            let Ok(occurrences) = get_occurrences(
                rrule,
                &start,
                event.timezone.as_deref(),
                event.exdates.as_deref().unwrap_or_default(),
                &search_start,
                &range.end_instant,
            ) else {
                return;
            };
            for occ_start in occurrences {
                let key = format!("{}@{}", event.id, occ_start.to_rfc3339());
                if exceptions.contains(&key) {
                    continue;
                }
                let occ_end = occ_start.checked_add_signed(duration).unwrap_or(occ_start);
                if occ_end > range.start_instant && occ_start < range.end_instant {
                    let occ = EventTiming::Timed {
                        start: occ_start,
                        end: occ_end,
                        timezone: timezone.clone(),
                    };
                    let start = canonical_start(&occ);
                    instances.push(build_instance(event, occ, key, start, true));
                }
            }
        }
    }
}

fn build_instance(
    event: &AppEvent,
    timing: EventTiming,
    occurrence_key: String,
    occurrence_start: CalendarDateTime,
    recurring: bool,
) -> EventInstance {
    EventInstance {
        id: event.id.clone(),
        occurrence_key,
        occurrence_start,
        title: event.title.clone(),
        description: event.description.clone(),
        timing,
        recurring,
        rrule: event.rrule.clone(),
        calendar_id: event.remote_collection_id.clone(),
        color: event.color.clone(),
        status: event.status.clone().unwrap_or(EventStatus::Confirmed),
        task_id: event.task_id.clone(),
        task_status: None,
    }
}

fn overlaps(timing: &EventTiming, range: &RenderWindow) -> bool {
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

#[cfg(test)]
mod tests;
