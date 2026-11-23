use crate::features::calendar::domain::{
    AppEvent, AppEventFilterColumn, AppTask, AppTaskFilterColumn, EventStatus, FilterOperator,
    TaskPriority,
};
use chrono::NaiveDate;

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
            date_text_matches(task.start.as_deref(), value, operator, today)
        }
        AppTaskFilterColumn::Due => date_text_matches(task.due.as_deref(), value, operator, today),
        AppTaskFilterColumn::Added => {
            date_text_matches(task.added.as_deref(), value, operator, today)
        }
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
    today: NaiveDate,
) -> bool {
    let Some(date) = date else {
        return false;
    };
    let needle = value_as_string(value);
    if matches!(
        operator,
        FilterOperator::Contains | FilterOperator::DoesNotContain
    ) {
        return date.to_lowercase().contains(&needle.to_lowercase());
    }
    let (Some(day), Some(needle_day)) = (date_prefix(date), resolve_filter_date(&needle, today))
    else {
        return false;
    };
    match operator {
        FilterOperator::Is | FilterOperator::IsNot => day == needle_day,
        FilterOperator::Before => day < needle_day,
        FilterOperator::After => day > needle_day,
        FilterOperator::OnOrBefore => day <= needle_day,
        FilterOperator::OnOrAfter => day >= needle_day,
        FilterOperator::Contains | FilterOperator::DoesNotContain => false,
    }
}

/// The calendar day of a canonical date or date-time string.
fn date_prefix(raw: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(raw.get(..10)?, "%Y-%m-%d").ok()
}

/// Resolve an absolute `YYYY-MM-DD` value or a relative day keyword.
pub(super) fn resolve_filter_date(raw: &str, today: NaiveDate) -> Option<NaiveDate> {
    let trimmed = raw.trim();
    match trimmed.to_lowercase().as_str() {
        "today" => Some(today),
        "tomorrow" => today.succ_opt(),
        "yesterday" => today.pred_opt(),
        _ => date_prefix(trimmed),
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
