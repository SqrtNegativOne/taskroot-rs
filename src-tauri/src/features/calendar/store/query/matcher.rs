use crate::features::calendar::domain::{
    AppEvent, AppEventFilter, AppTask, AppTaskFilter, FilterGroup, FilterNode, FilterOperator,
    LogicalOperator,
};
use chrono::NaiveDate;
use serde_json::Value;

use super::columns::{event_column_matches, task_column_matches};

pub(super) fn task_matches_filters(
    task: &AppTask,
    filters: &FilterGroup<AppTaskFilter>,
    today: NaiveDate,
) -> bool {
    match_group(filters.operator, &filters.children, &|filter| {
        match_condition(
            filter.column.as_ref(),
            filter.value.as_ref(),
            filter.operator.as_ref(),
            |column, value, operator| task_column_matches(task, column, value, operator, today),
        )
    })
    .unwrap_or(true)
}

pub(super) fn event_matches_filters(
    event: &AppEvent,
    filters: &FilterGroup<AppEventFilter>,
) -> bool {
    match_group(filters.operator, &filters.children, &|filter| {
        match_condition(
            filter.column.as_ref(),
            filter.value.as_ref(),
            filter.operator.as_ref(),
            |column, value, operator| event_column_matches(event, column, value, operator),
        )
    })
    .unwrap_or(true)
}

// None represents unfinished conditions, not a matching branch in an OR group.
fn match_group<F>(
    operator: LogicalOperator,
    children: &[FilterNode<F>],
    match_leaf: &impl Fn(&F) -> Option<bool>,
) -> Option<bool> {
    let mut results = children.iter().filter_map(|child| match child {
        FilterNode::Condition { filter } => match_leaf(filter),
        FilterNode::Group { operator, children } => match_group(*operator, children, match_leaf),
    });
    let first = results.next()?;
    Some(match operator {
        LogicalOperator::And => first && results.all(|matched| matched),
        LogicalOperator::Or => first || results.any(|matched| matched),
    })
}

fn match_condition<C>(
    column: Option<&C>,
    value: Option<&Value>,
    operator: Option<&FilterOperator>,
    match_value: impl Fn(&C, &Value, &FilterOperator) -> bool,
) -> Option<bool> {
    let column = column?;
    let value = value?;
    let operator = operator.unwrap_or(&FilterOperator::Is);
    let values: &[Value] = match value {
        Value::Array(values) => values,
        value => std::slice::from_ref(value),
    };
    let mut candidates = values
        .iter()
        .filter(|value| !value.is_null() && !value.as_str().is_some_and(str::is_empty));
    let first = candidates.next()?;
    let matched = match_value(column, first, operator)
        || candidates.any(|candidate| match_value(column, candidate, operator));
    Some(
        if matches!(
            operator,
            FilterOperator::IsNot | FilterOperator::DoesNotContain
        ) {
            !matched
        } else {
            matched
        },
    )
}
