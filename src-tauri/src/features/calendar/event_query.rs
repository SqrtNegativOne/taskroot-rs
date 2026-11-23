//! Filtered event projection, preserving recurrence context independently of selection.

use super::domain::{AppEvent, AppEventFilter, AppEventSort, EventInstance, FilterGroup};
use super::occurrences::{expand_selected_event_instances, single_instance};
use super::store::query;

#[must_use]
pub fn query_event_instances(
    events: &[AppEvent],
    filters: &FilterGroup<AppEventFilter>,
    sorts: &[AppEventSort],
    search: &str,
    range: Option<(&str, &str)>,
) -> Vec<EventInstance> {
    let selected = query::query_events(events, filters, sorts, search);
    match range {
        Some((start, end)) => expand_selected_event_instances(events, &selected, start, end),
        None => selected.iter().filter_map(single_instance).collect(),
    }
}

#[cfg(test)]
mod tests;
