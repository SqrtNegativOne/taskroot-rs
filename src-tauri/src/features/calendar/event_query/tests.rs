use super::*;
use crate::features::calendar::domain::{
    AppEventFilterColumn, EventStatus, FilterNode, FilterOperator, LogicalOperator, SortDirection,
};
use crate::features::calendar::ics::read_event_resource;
use crate::test_support::ics::{series, with_exception};
use serde_json::json;

const RANGE: Option<(&str, &str)> = Some(("2026-11-02T00:00:00Z", "2026-11-02T23:59:59Z"));

fn rows() -> Vec<AppEvent> {
    let mut events =
        read_event_resource(&with_exception(series()), "/series.ics", "/events/").unwrap();
    events[0].title = "Master".into();
    events[1].title = "Moved".into();
    events
}

fn title(value: &str) -> FilterGroup<AppEventFilter> {
    FilterGroup::from(vec![AppEventFilter {
        column: Some(AppEventFilterColumn::Title),
        operator: Some(FilterOperator::Is),
        value: Some(json!([value])),
    }])
}

#[test]
fn a_filtered_out_override_does_not_resurrect_the_original_slot() {
    let instances = query_event_instances(&rows(), &title("Master"), &[], "", RANGE);
    assert!(
        instances.is_empty(),
        "the original slot remains suppressed even when its replacement does not match"
    );
}

#[test]
fn a_matching_override_inherits_the_filtered_out_master_rule() {
    let events = rows();
    let instances = query_event_instances(&events, &title("Moved"), &[], "", RANGE);
    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0].id, events[1].id);
    assert_eq!(instances[0].rrule, events[0].rrule);
    assert_eq!(instances[0].occurrence_start, "2026-11-02T11:00:00");
}

#[test]
fn nested_filters_and_search_do_not_discard_cancellation_tombstones() {
    let mut events = rows();
    events[1].status = Some(EventStatus::Cancelled);
    let filters = FilterGroup {
        operator: LogicalOperator::And,
        children: vec![
            FilterNode::Group {
                operator: LogicalOperator::Or,
                children: title("Master").children,
            },
            FilterNode::Condition {
                filter: AppEventFilter {
                    column: Some(AppEventFilterColumn::Status),
                    operator: Some(FilterOperator::Is),
                    value: Some(json!(["confirmed"])),
                },
            },
        ],
    };
    let filtered = query_event_instances(&events, &filters, &[], "", RANGE);
    assert!(
        filtered.is_empty(),
        "cancelled occurrence reappeared: {filtered:?}"
    );
    let searched = query_event_instances(&events, &FilterGroup::default(), &[], "Master", RANGE);
    assert!(
        searched.is_empty(),
        "search resurrected a cancellation: {searched:?}"
    );
}

#[test]
fn filtered_all_day_cancellations_keep_the_original_date_suppressed() {
    let mut events = rows();
    for event in &mut events {
        event.start_time = "2026-11-02".into();
        event.end_time = "2026-11-03".into();
        event.is_all_day = Some(true);
    }
    events[1].original_start_time = Some("2026-11-02".into());
    events[1].status = Some(EventStatus::Cancelled);
    let instances = query_event_instances(
        &events,
        &title("Master"),
        &[],
        "",
        Some(("2026-11-02", "2026-11-02")),
    );
    assert!(
        instances.is_empty(),
        "cancelled all-day date reappeared: {instances:?}"
    );
}

#[test]
fn filtered_projection_preserves_row_sorting_and_range_less_behavior() {
    let events = rows();
    let sort = [AppEventSort {
        column: Some(AppEventFilterColumn::Title),
        direction: Some(SortDirection::Desc),
    }];
    let range = Some(("2026-10-19T00:00:00Z", "2026-11-02T23:59:59Z"));
    let expanded = query_event_instances(&events, &FilterGroup::default(), &sort, "", range);
    assert_eq!(
        expanded
            .iter()
            .map(|instance| instance.title.as_str())
            .collect::<Vec<_>>(),
        ["Moved", "Master", "Master"]
    );
    let unexpanded = query_event_instances(&events, &FilterGroup::default(), &sort, "", None);
    assert_eq!(unexpanded.len(), 2);
    assert_eq!(unexpanded[0].id, events[1].id);
    assert_eq!(unexpanded[1].occurrence_start, "2026-10-19T11:00:00");
}
