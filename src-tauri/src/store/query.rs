//! In-memory filtering, sorting and search over the store projections.
//!
//! These replace the SQL `QueryBuilder` paths; the schema in `domain::filters`
//! still describes which columns the frontend may filter and sort by.

use crate::domain::{
    AppEvent, AppEventFilter, AppEventFilterColumn, AppTask, AppTaskFilter, AppTaskFilterColumn,
    AppTaskSort, EventStatus, FilterOperator, SortDirection, TaskPriority,
};
use std::cmp::Ordering;

#[must_use]
pub fn query_tasks(
    tasks: &[AppTask],
    filters: &[AppTaskFilter],
    sorts: &[AppTaskSort],
    search: &str,
) -> Vec<AppTask> {
    let needle = search.trim().to_lowercase();
    let mut result: Vec<AppTask> = tasks
        .iter()
        .filter(|task| task_matches_filters(task, filters))
        .filter(|task| task_matches_search(task, &needle))
        .cloned()
        .collect();
    result.sort_by(|a, b| compare_tasks(a, b, sorts));
    result
}

#[must_use]
pub fn query_events(
    events: &[AppEvent],
    filters: &[AppEventFilter],
    search: &str,
) -> Vec<AppEvent> {
    let needle = search.trim().to_lowercase();
    events
        .iter()
        .filter(|event| event_matches_filters(event, filters))
        .filter(|event| event_matches_search(event, &needle))
        .cloned()
        .collect()
}

fn task_matches_filters(task: &AppTask, filters: &[AppTaskFilter]) -> bool {
    filters.iter().all(|filter| {
        let (Some(column), Some(value)) = (&filter.column, &filter.value) else {
            return true;
        };
        let operator = filter.operator.clone().unwrap_or(FilterOperator::Is);
        let candidates = filter_values(value);
        if candidates.is_empty() {
            return true;
        }
        let matched = candidates
            .iter()
            .any(|candidate| task_column_matches(task, column, candidate, &operator));
        if is_exclusion(&operator) {
            !matched
        } else {
            matched
        }
    })
}

fn event_matches_filters(event: &AppEvent, filters: &[AppEventFilter]) -> bool {
    filters.iter().all(|filter| {
        let (Some(column), Some(value)) = (&filter.column, &filter.value) else {
            return true;
        };
        let operator = filter.operator.clone().unwrap_or(FilterOperator::Is);
        let candidates = filter_values(value);
        if candidates.is_empty() {
            return true;
        }
        let matched = candidates
            .iter()
            .any(|candidate| event_column_matches(event, column, candidate, &operator));
        if is_exclusion(&operator) {
            !matched
        } else {
            matched
        }
    })
}

const fn is_exclusion(operator: &FilterOperator) -> bool {
    matches!(
        operator,
        FilterOperator::IsNot | FilterOperator::DoesNotContain
    )
}

fn filter_values(value: &serde_json::Value) -> Vec<&serde_json::Value> {
    match value {
        serde_json::Value::Array(values) => values.iter().collect(),
        other => vec![other],
    }
}

fn task_column_matches(
    task: &AppTask,
    column: &AppTaskFilterColumn,
    value: &serde_json::Value,
    operator: &FilterOperator,
) -> bool {
    match column {
        AppTaskFilterColumn::Id => text_matches(task.id.as_str(), value, operator),
        AppTaskFilterColumn::Title => text_matches(&task.title, value, operator),
        AppTaskFilterColumn::Status => task
            .status
            .as_ref()
            .is_some_and(|status| status_name(status) == value_as_string(value)),
        AppTaskFilterColumn::Priority => {
            let value = value
                .as_i64()
                .or_else(|| value.as_str().and_then(|raw| raw.parse().ok()));
            value.is_some_and(|number| number == priority_number(task.priority.as_ref()))
        }
        AppTaskFilterColumn::Tags => task.tags.as_ref().is_some_and(|tags| {
            let needle = value_as_string(value).to_lowercase();
            tags.iter()
                .any(|tag| tag.name.to_lowercase().contains(&needle))
        }),
        AppTaskFilterColumn::Start => date_text_matches(task.start.as_deref(), value, operator),
        AppTaskFilterColumn::Due => date_text_matches(task.due.as_deref(), value, operator),
        AppTaskFilterColumn::Remotecollectionid => task
            .remote_collection_id
            .as_ref()
            .is_some_and(|id| id.as_str() == value_as_string(value)),
    }
}

fn date_text_matches(
    date: Option<&str>,
    value: &serde_json::Value,
    operator: &FilterOperator,
) -> bool {
    let Some(date) = date else {
        return false;
    };
    let needle = value_as_string(value);
    match operator {
        FilterOperator::Contains | FilterOperator::DoesNotContain => {
            date.to_lowercase().contains(&needle.to_lowercase())
        }
        FilterOperator::Is | FilterOperator::IsNot => date == needle,
    }
}

fn event_column_matches(
    event: &AppEvent,
    column: &AppEventFilterColumn,
    value: &serde_json::Value,
    operator: &FilterOperator,
) -> bool {
    match column {
        AppEventFilterColumn::Title => text_matches(&event.title, value, operator),
        AppEventFilterColumn::Status => event
            .status
            .as_ref()
            .is_some_and(|status| event_status_name(status) == value_as_string(value)),
        AppEventFilterColumn::Remotecollectionid => event
            .remote_collection_id
            .as_ref()
            .is_some_and(|id| id.as_str() == value_as_string(value)),
    }
}

fn text_matches(haystack: &str, value: &serde_json::Value, operator: &FilterOperator) -> bool {
    let needle = value_as_string(value);
    match operator {
        FilterOperator::Contains | FilterOperator::DoesNotContain => {
            haystack.to_lowercase().contains(&needle.to_lowercase())
        }
        FilterOperator::Is | FilterOperator::IsNot => haystack == needle,
    }
}

fn value_as_string(value: &serde_json::Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), ToString::to_string)
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
}

fn compare_tasks(a: &AppTask, b: &AppTask, sorts: &[AppTaskSort]) -> Ordering {
    for sort in sorts {
        let Some(column) = &sort.column else {
            continue;
        };
        let ordering = compare_task_column(a, b, column);
        let ordering = match sort.direction {
            Some(SortDirection::Desc) => ordering.reverse(),
            _ => ordering,
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    a.id.as_str().cmp(b.id.as_str())
}

fn compare_task_column(a: &AppTask, b: &AppTask, column: &AppTaskFilterColumn) -> Ordering {
    match column {
        AppTaskFilterColumn::Id => a.id.as_str().cmp(b.id.as_str()),
        AppTaskFilterColumn::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
        AppTaskFilterColumn::Status => {
            status_rank(a.status.as_ref()).cmp(&status_rank(b.status.as_ref()))
        }
        AppTaskFilterColumn::Priority => {
            priority_number(a.priority.as_ref()).cmp(&priority_number(b.priority.as_ref()))
        }
        AppTaskFilterColumn::Start => a
            .start
            .as_deref()
            .unwrap_or("9999")
            .cmp(b.start.as_deref().unwrap_or("9999")),
        AppTaskFilterColumn::Due => a
            .due
            .as_deref()
            .unwrap_or("9999")
            .cmp(b.due.as_deref().unwrap_or("9999")),
        AppTaskFilterColumn::Tags => tag_names(a).cmp(&tag_names(b)),
        AppTaskFilterColumn::Remotecollectionid => collection_id(a).cmp(collection_id(b)),
    }
}

fn collection_id(task: &AppTask) -> &str {
    task.remote_collection_id
        .as_ref()
        .map_or("", |id| id.as_str())
}

fn tag_names(task: &AppTask) -> String {
    task.tags.as_ref().map_or_else(String::new, |tags| {
        tags.iter().map(|tag| tag.name.as_str()).collect::<Vec<_>>().join(",")
    })
}

const fn status_rank(status: Option<&crate::domain::AppTaskStatus>) -> u8 {
    match status {
        Some(crate::domain::AppTaskStatus::Todo) => 0,
        Some(crate::domain::AppTaskStatus::NextUp) => 1,
        Some(crate::domain::AppTaskStatus::Doing) => 2,
        Some(crate::domain::AppTaskStatus::Done) => 3,
        None => 4,
    }
}

const fn priority_number(priority: Option<&TaskPriority>) -> i64 {
    match priority {
        Some(TaskPriority::Low) => 1,
        Some(TaskPriority::Medium) => 2,
        Some(TaskPriority::High) => 3,
        Some(TaskPriority::Urgent) => 4,
        Some(TaskPriority::None) | None => 0,
    }
}

const fn status_name(status: &crate::domain::AppTaskStatus) -> &'static str {
    match status {
        crate::domain::AppTaskStatus::Todo => "todo",
        crate::domain::AppTaskStatus::NextUp => "next-up",
        crate::domain::AppTaskStatus::Doing => "doing",
        crate::domain::AppTaskStatus::Done => "done",
    }
}

const fn event_status_name(status: &EventStatus) -> &'static str {
    match status {
        EventStatus::Confirmed => "confirmed",
        EventStatus::Tentative => "tentative",
        EventStatus::Cancelled => "cancelled",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Tag, TaskId};
    use serde_json::json;

    fn task(id: &str, title: &str) -> AppTask {
        AppTask {
            id: TaskId::from(id),
            title: title.to_string(),
            status: Some(crate::domain::AppTaskStatus::Todo),
            priority: Some(TaskPriority::Medium),
            tags: None,
            parent_task: None,
            dependencies: None,
            est: None,
            added: None,
            canvas_x: None,
            canvas_y: None,
            on_canvas: None,
            remote_id: None,
            remote_collection_id: None,
            notes: None,
            tabs: None,
            start: None,
            due: None,
            completed_at: None,
            rrule: None,
            geo: None,
            alarms: None,
            timezone: None,
            updated_at: None,
            etag: None,
        }
    }

    fn filter(column: AppTaskFilterColumn, operator: FilterOperator, value: serde_json::Value) -> AppTaskFilter {
        AppTaskFilter {
            column: Some(column),
            operator: Some(operator),
            value: Some(value),
        }
    }

    #[test]
    fn status_filter_includes_and_excludes() {
        let mut done = task("b", "Done");
        done.status = Some(crate::domain::AppTaskStatus::Done);
        let tasks = vec![task("a", "Open"), done];

        let open = query_tasks(
            &tasks,
            &[filter(
                AppTaskFilterColumn::Status,
                FilterOperator::IsNot,
                json!(["done"]),
            )],
            &[],
            "",
        );
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].id.as_str(), "a");
    }

    #[test]
    fn tags_filter_matches_by_name() {
        let mut tagged = task("a", "Tagged");
        tagged.tags = Some(vec![Tag {
            id: "t".into(),
            name: "work".into(),
            color: None,
        }]);
        let tasks = vec![tagged, task("b", "Plain")];

        let matched = query_tasks(
            &tasks,
            &[filter(
                AppTaskFilterColumn::Tags,
                FilterOperator::Is,
                json!(["work"]),
            )],
            &[],
            "",
        );
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].id.as_str(), "a");
    }

    #[test]
    fn title_sort_is_case_insensitive_and_ties_break_by_id() {
        let tasks = vec![task("b", "beta"), task("a", "Alpha"), task("c", "alpha")];
        let sorted = query_tasks(
            &tasks,
            &[],
            &[AppTaskSort {
                column: Some(AppTaskFilterColumn::Title),
                direction: Some(SortDirection::Asc),
            }],
            "",
        );
        let ids: Vec<&str> = sorted.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "c", "b"]);
    }

    #[test]
    fn search_matches_title_and_tag_names() {
        let mut tagged = task("a", "Something");
        tagged.tags = Some(vec![Tag {
            id: "t".into(),
            name: "errand".into(),
            color: None,
        }]);
        let tasks = vec![tagged, task("b", "Other")];

        let matched = query_tasks(&tasks, &[], &[], "errand");
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].id.as_str(), "a");
    }
}
