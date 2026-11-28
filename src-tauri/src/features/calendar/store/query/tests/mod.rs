use super::*;
use crate::features::calendar::domain::{
    AppEventFilterColumn, AppTaskFilterColumn, EventStatus, FilterOperator, SortDirection, Tag,
    TaskId, TaskPriority,
};
use serde_json::json;

mod empty_fields;
mod events;
mod recursive;
mod tasks;

pub(super) fn task(id: &str, title: &str) -> AppTask {
    AppTask {
        id: TaskId::from(id),
        title: title.to_string(),
        status: Some(crate::features::calendar::domain::AppTaskStatus::Todo),
        priority: Some(TaskPriority::Medium),
        tags: None,
        parent_task: None,
        dependencies: None,
        est: None,
        time_spent: None,
        added: None,
        remote_id: None,
        remote_collection_id: None,
        notes: None,
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

pub(super) fn filter(
    column: AppTaskFilterColumn,
    operator: FilterOperator,
    value: serde_json::Value,
) -> AppTaskFilter {
    AppTaskFilter {
        column: Some(column),
        operator: Some(operator),
        value: Some(value),
    }
}

pub(super) fn event(id: &str, title: &str) -> AppEvent {
    AppEvent {
        id: id.into(),
        remote_id: None,
        remote_collection_id: None,
        task_id: None,
        title: title.to_string(),
        description: None,
        priority: None,
        tags: None,
        start_time: "2026-09-07T09:00:00".parse().unwrap(),
        end_time: "2026-09-07T10:00:00".parse().unwrap(),
        rrule: None,
        exdates: None,
        recurring_event_id: None,
        original_start_time: None,
        status: Some(EventStatus::Confirmed),
        updated_at: None,
        added: None,
        color: None,
        etag: None,
        is_all_day: Some(false),
        timezone: None,
    }
}
