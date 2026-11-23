use super::*;

#[test]
fn event_priority_and_tag_filters_match() {
    let mut urgent = event("a", "Urgent");
    urgent.priority = Some(TaskPriority::Urgent);
    urgent.tags = Some(vec![Tag {
        id: "t".into(),
        name: "work".into(),
        color: None,
    }]);
    let events = vec![urgent, event("b", "Plain")];

    let priority = query_events(
        &events,
        &FilterGroup::from(vec![AppEventFilter {
            column: Some(AppEventFilterColumn::Priority),
            operator: Some(FilterOperator::Is),
            value: Some(json!(4)),
        }]),
        &[],
        "",
    );
    assert_eq!(priority.len(), 1);
    assert_eq!(priority[0].id.as_str(), "a");

    let tags = query_events(
        &events,
        &FilterGroup::from(vec![AppEventFilter {
            column: Some(AppEventFilterColumn::Tags),
            operator: Some(FilterOperator::Is),
            value: Some(json!(["work"])),
        }]),
        &[],
        "",
    );
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].id.as_str(), "a");
}

#[test]
fn event_priority_sort_is_numeric() {
    let mut urgent = event("a", "Urgent");
    urgent.priority = Some(TaskPriority::Urgent);
    let mut low = event("b", "Low");
    low.priority = Some(TaskPriority::Low);
    let events = vec![urgent, low, event("c", "None")];

    let sorted = query_events(
        &events,
        &FilterGroup::default(),
        &[AppEventSort {
            column: Some(AppEventFilterColumn::Priority),
            direction: Some(SortDirection::Desc),
        }],
        "",
    );
    let ids: Vec<&str> = sorted.iter().map(|event| event.id.as_str()).collect();
    assert_eq!(ids, vec!["a", "b", "c"]);
}
