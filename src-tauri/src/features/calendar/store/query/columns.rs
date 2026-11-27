use crate::features::calendar::domain::{
    AppEvent, AppEventFilterColumn, AppTask, AppTaskFilterColumn, CalendarDateTime, EventStatus,
    FilterOperator, TaskPriority,
};
use chrono::{DateTime, NaiveDate, Utc};
use std::str::FromStr;

pub(super) fn task_column_is_empty(task: &AppTask, column: &AppTaskFilterColumn) -> bool {
    match column {
        AppTaskFilterColumn::Id => task.id.as_str().is_empty(),
        AppTaskFilterColumn::Title => task.title.is_empty(),
        AppTaskFilterColumn::Status => task.status.is_none(),
        AppTaskFilterColumn::Priority => priority_number(task.priority.as_ref()) == 0,
        AppTaskFilterColumn::Tags => task.tags.as_ref().is_none_or(Vec::is_empty),
        AppTaskFilterColumn::Start => task.start.is_none(),
        AppTaskFilterColumn::Due => task.due.is_none(),
        AppTaskFilterColumn::Added => task.added.is_none(),
        AppTaskFilterColumn::Remotecollectionid => {
            text_is_empty(task.remote_collection_id.as_ref().map(|id| id.as_str()))
        }
    }
}

pub(super) fn event_column_is_empty(event: &AppEvent, column: &AppEventFilterColumn) -> bool {
    match column {
        AppEventFilterColumn::Title => event.title.is_empty(),
        AppEventFilterColumn::Status => event.status.is_none(),
        AppEventFilterColumn::Priority => priority_number(event.priority.as_ref()) == 0,
        AppEventFilterColumn::Tags => event.tags.as_ref().is_none_or(Vec::is_empty),
        AppEventFilterColumn::Remotecollectionid => {
            text_is_empty(event.remote_collection_id.as_ref().map(|id| id.as_str()))
        }
    }
}

fn text_is_empty(value: Option<&str>) -> bool {
    value.is_none_or(str::is_empty)
}

pub(super) fn task_column_matches(
    task: &AppTask,
    column: &AppTaskFilterColumn,
    value: &serde_json::Value,
    operator: &FilterOperator,
    today: NaiveDate,
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
        AppTaskFilterColumn::Start => {
            calendar_date_matches(task.start, value, operator, today)
        }
        AppTaskFilterColumn::Due => calendar_date_matches(task.due, value, operator, today),
        AppTaskFilterColumn::Added => timestamp_matches(task.added, value, operator, today),
        AppTaskFilterColumn::Remotecollectionid => task
            .remote_collection_id
            .as_ref()
            .is_some_and(|id| id.as_str() == value_as_string(value)),
    }
}

fn calendar_date_matches(
    date: Option<CalendarDateTime>,
    value: &serde_json::Value,
    operator: &FilterOperator,
    today: NaiveDate,
) -> bool {
    let Some(date) = date else {
        return false;
    };
    date_day_matches(date.date(), || date.to_string(), value, operator, today)
}

fn timestamp_matches(
    stamp: Option<DateTime<Utc>>,
    value: &serde_json::Value,
    operator: &FilterOperator,
    today: NaiveDate,
) -> bool {
    let Some(stamp) = stamp else {
        return false;
    };
    date_day_matches(
        stamp.date_naive(),
        || stamp.to_rfc3339(),
        value,
        operator,
        today,
    )
}

fn date_day_matches(
    day: NaiveDate,
    as_text: impl FnOnce() -> String,
    value: &serde_json::Value,
    operator: &FilterOperator,
    today: NaiveDate,
) -> bool {
    let needle = value_as_string(value);
    if matches!(
        operator,
        FilterOperator::Contains | FilterOperator::DoesNotContain
    ) {
        return as_text().to_lowercase().contains(&needle.to_lowercase());
    }
    let Some(needle_day) = resolve_filter_date(&needle, today) else {
        return false;
    };
    match operator {
        FilterOperator::Is | FilterOperator::IsNot => day == needle_day,
        FilterOperator::Before => day < needle_day,
        FilterOperator::After => day > needle_day,
        FilterOperator::OnOrBefore => day <= needle_day,
        FilterOperator::OnOrAfter => day >= needle_day,
        FilterOperator::Contains
        | FilterOperator::DoesNotContain
        | FilterOperator::IsEmpty
        | FilterOperator::IsNotEmpty => false,
    }
}

/// Resolve an absolute `YYYY-MM-DD` / date-time value or a relative day keyword.
pub(super) fn resolve_filter_date(raw: &str, today: NaiveDate) -> Option<NaiveDate> {
    let trimmed = raw.trim();
    match trimmed.to_lowercase().as_str() {
        "today" => Some(today),
        "tomorrow" => today.succ_opt(),
        "yesterday" => today.pred_opt(),
        _ => CalendarDateTime::from_str(trimmed)
            .ok()
            .map(CalendarDateTime::date),
    }
}

pub(super) fn event_column_matches(
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
        AppEventFilterColumn::Priority => {
            let value = value
                .as_i64()
                .or_else(|| value.as_str().and_then(|raw| raw.parse().ok()));
            value.is_some_and(|number| number == priority_number(event.priority.as_ref()))
        }
        AppEventFilterColumn::Tags => event.tags.as_ref().is_some_and(|tags| {
            let needle = value_as_string(value).to_lowercase();
            tags.iter()
                .any(|tag| tag.name.to_lowercase().contains(&needle))
        }),
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
        FilterOperator::Before
        | FilterOperator::After
        | FilterOperator::OnOrBefore
        | FilterOperator::OnOrAfter
        | FilterOperator::Is
        | FilterOperator::IsNot => haystack == needle,
        FilterOperator::IsEmpty | FilterOperator::IsNotEmpty => false,
    }
}

fn value_as_string(value: &serde_json::Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), ToString::to_string)
}

pub(super) const fn priority_number(priority: Option<&TaskPriority>) -> i64 {
    match priority {
        Some(TaskPriority::Low) => 1,
        Some(TaskPriority::Medium) => 2,
        Some(TaskPriority::High) => 3,
        Some(TaskPriority::Urgent) => 4,
        Some(TaskPriority::None) | None => 0,
    }
}

const fn status_name(status: &crate::features::calendar::domain::AppTaskStatus) -> &'static str {
    match status {
        crate::features::calendar::domain::AppTaskStatus::Todo => "todo",
        crate::features::calendar::domain::AppTaskStatus::NextUp => "next-up",
        crate::features::calendar::domain::AppTaskStatus::Done => "done",
        crate::features::calendar::domain::AppTaskStatus::Cancelled => "cancelled",
    }
}

const fn event_status_name(status: &EventStatus) -> &'static str {
    match status {
        EventStatus::Confirmed => "confirmed",
        EventStatus::Tentative => "tentative",
        EventStatus::Cancelled => "cancelled",
    }
}
