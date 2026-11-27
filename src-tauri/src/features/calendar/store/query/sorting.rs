use crate::features::calendar::domain::{
    AppEvent, AppEventFilterColumn, AppEventSort, AppTask, AppTaskFilterColumn, AppTaskSort,
    EventStatus, SortDirection, Tag,
};
use std::cmp::Ordering;

use super::columns::priority_number;

pub(super) fn compare_events(a: &AppEvent, b: &AppEvent, sorts: &[AppEventSort]) -> Ordering {
    for sort in sorts {
        let Some(column) = &sort.column else {
            continue;
        };
        let ordering = compare_event_column(a, b, column);
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

fn compare_event_column(a: &AppEvent, b: &AppEvent, column: &AppEventFilterColumn) -> Ordering {
    match column {
        AppEventFilterColumn::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
        AppEventFilterColumn::Status => {
            event_status_rank(a.status.as_ref()).cmp(&event_status_rank(b.status.as_ref()))
        }
        AppEventFilterColumn::Priority => {
            priority_number(a.priority.as_ref()).cmp(&priority_number(b.priority.as_ref()))
        }
        AppEventFilterColumn::Tags => {
            tag_names(a.tags.as_deref()).cmp(&tag_names(b.tags.as_deref()))
        }
        AppEventFilterColumn::Remotecollectionid => {
            event_collection_id(a).cmp(event_collection_id(b))
        }
    }
}

pub(super) fn compare_tasks(a: &AppTask, b: &AppTask, sorts: &[AppTaskSort]) -> Ordering {
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
        AppTaskFilterColumn::Start => compare_opt(a.start, b.start),
        AppTaskFilterColumn::Due => compare_opt(a.due, b.due),
        AppTaskFilterColumn::Added => compare_opt(a.added, b.added),
        AppTaskFilterColumn::Tags => {
            tag_names(a.tags.as_deref()).cmp(&tag_names(b.tags.as_deref()))
        }
        AppTaskFilterColumn::Remotecollectionid => collection_id(a).cmp(collection_id(b)),
    }
}

fn compare_opt<T: Ord>(a: Option<T>, b: Option<T>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn collection_id(task: &AppTask) -> &str {
    task.remote_collection_id
        .as_ref()
        .map_or("", |id| id.as_str())
}

fn event_collection_id(event: &AppEvent) -> &str {
    event
        .remote_collection_id
        .as_ref()
        .map_or("", |id| id.as_str())
}

fn tag_names(tags: Option<&[Tag]>) -> String {
    tags.map_or_else(String::new, |tags| {
        tags.iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>()
            .join(",")
    })
}

const fn status_rank(status: Option<&crate::features::calendar::domain::AppTaskStatus>) -> u8 {
    match status {
        Some(crate::features::calendar::domain::AppTaskStatus::Todo) => 0,
        Some(crate::features::calendar::domain::AppTaskStatus::NextUp) => 1,
        Some(crate::features::calendar::domain::AppTaskStatus::Done) => 2,
        Some(crate::features::calendar::domain::AppTaskStatus::Cancelled) => 3,
        None => 4,
    }
}

const fn event_status_rank(status: Option<&EventStatus>) -> u8 {
    match status {
        Some(EventStatus::Confirmed) => 0,
        Some(EventStatus::Tentative) => 1,
        Some(EventStatus::Cancelled) => 2,
        None => 3,
    }
}
