//! In-memory filtering, sorting and search over the store projections.
//!
//! These replace the SQL `QueryBuilder` paths; the schema in `domain::filters`
//! still describes which columns the frontend may filter and sort by.

use crate::features::calendar::domain::{
    AppEvent, AppEventFilter, AppEventSort, AppTask, AppTaskFilter, AppTaskSort, FilterGroup,
};
use chrono::Local;

mod columns;
mod matcher;
mod sorting;

use matcher::{event_matches_filters, task_matches_filters};
use sorting::{compare_events, compare_tasks};

#[must_use]
pub fn query_tasks(
    tasks: &[AppTask],
    filters: &FilterGroup<AppTaskFilter>,
    sorts: &[AppTaskSort],
    search: &str,
) -> Vec<AppTask> {
    let needle = search.trim().to_lowercase();
    let today = Local::now().date_naive();
    let mut result: Vec<AppTask> = tasks
        .iter()
        .filter(|task| task_matches_filters(task, filters, today))
        .filter(|task| task_matches_search(task, &needle))
        .cloned()
        .collect();
    result.sort_by(|a, b| compare_tasks(a, b, sorts));
    result
}

#[must_use]
pub fn query_events(
    events: &[AppEvent],
    filters: &FilterGroup<AppEventFilter>,
    sorts: &[AppEventSort],
    search: &str,
) -> Vec<AppEvent> {
    let needle = search.trim().to_lowercase();
    let mut result: Vec<AppEvent> = events
        .iter()
        .filter(|event| event_matches_filters(event, filters))
        .filter(|event| event_matches_search(event, &needle))
        .cloned()
        .collect();
    result.sort_by(|a, b| compare_events(a, b, sorts));
    result
}

fn task_matches_search(task: &AppTask, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    task.title.to_lowercase().contains(needle)
        || task.tags.as_ref().is_some_and(|tags| {
            tags.iter()
                .any(|tag| tag.name.to_lowercase().contains(needle))
        })
}

fn event_matches_search(event: &AppEvent, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    event.title.to_lowercase().contains(needle)
        || event
            .description
            .as_ref()
            .is_some_and(|description| description.to_lowercase().contains(needle))
        || event.tags.as_ref().is_some_and(|tags| {
            tags.iter()
                .any(|tag| tag.name.to_lowercase().contains(needle))
        })
}

#[cfg(test)]
mod tests;
