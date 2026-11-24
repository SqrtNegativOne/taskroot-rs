use super::*;
use crate::features::calendar::domain::CollectionId;
use serde_json::Value;

fn unary_filters<F: serde::de::DeserializeOwned>(
    column: &str,
    operator: &str,
    value: Option<Value>,
) -> FilterGroup<F> {
    let mut leaf = json!({"column": column, "operator": operator});
    if let Some(value) = value {
        leaf["value"] = value;
    }
    serde_json::from_value(json!({
        "operator": "and", "children": [{"kind": "condition", "filter": leaf}]
    }))
    .unwrap()
}

fn task_ids(tasks: &[AppTask], filters: &FilterGroup<AppTaskFilter>) -> Vec<String> {
    query_tasks(tasks, filters, &[], "")
        .iter()
        .map(|task| task.id.as_str().to_owned())
        .collect()
}

fn event_ids(events: &[AppEvent], filters: &FilterGroup<AppEventFilter>) -> Vec<String> {
    query_events(events, filters, &[], "")
        .iter()
        .map(|event| event.id.as_str().to_owned())
        .collect()
}

#[test]
fn empty_text_matches_blank_titles_without_treating_it_as_an_unfinished_condition() {
    let tasks = [task("empty", ""), task("present", "Title")];
    let events = [event("empty", ""), event("present", "Title")];
    for (operator, expected) in [("is empty", ["empty"]), ("is not empty", ["present"])] {
        assert_eq!(
            task_ids(&tasks, &unary_filters("title", operator, None)),
            expected
        );
        assert_eq!(
            event_ids(&events, &unary_filters("title", operator, None)),
            expected
        );
    }
}

#[test]
fn empty_tags_include_absent_and_empty_lists_without_needing_a_value() {
    let absent = task("absent", "Absent tags");
    let mut empty = task("empty", "Empty tags");
    empty.tags = Some(vec![]);
    let mut tagged = task("tagged", "Work");
    tagged.tags = Some(vec![Tag {
        id: "work".into(),
        name: "work".into(),
        color: None,
    }]);
    let tasks = [absent, empty, tagged];
    for value in [
        None,
        Some(Value::Null),
        Some(json!([])),
        Some(json!(["stale"])),
    ] {
        assert_eq!(
            task_ids(&tasks, &unary_filters("tags", "is empty", value.clone())),
            ["absent", "empty"]
        );
        assert_eq!(
            task_ids(&tasks, &unary_filters("tags", "is not empty", value)),
            ["tagged"]
        );
    }
}

#[test]
fn optional_dates_distinguish_absent_or_blank_from_date_and_datetime_values() {
    for column in ["start", "due", "added"] {
        let absent = task("absent", "No date");
        let mut blank = task("blank", "Blank date");
        let mut date = task("date", "Date only");
        let mut timed = task("timed", "Timed date");
        for (task, value) in [
            (&mut blank, ""),
            (&mut date, "2026-11-20"),
            (&mut timed, "2026-11-20T09:00:00Z"),
        ] {
            let field = match column {
                "start" => &mut task.start,
                "due" => &mut task.due,
                _ => &mut task.added,
            };
            *field = Some(value.into());
        }
        let tasks = [absent, blank, date, timed];
        assert_eq!(
            task_ids(&tasks, &unary_filters(column, "is empty", None)),
            ["absent", "blank"],
            "{column}"
        );
        assert_eq!(
            task_ids(&tasks, &unary_filters(column, "is not empty", None)),
            ["date", "timed"],
            "{column}"
        );
    }
}

#[test]
fn priority_none_is_empty_whether_explicit_or_absent() {
    let mut absent = task("absent", "No priority");
    absent.priority = None;
    let mut none = task("none", "Explicit none");
    none.priority = Some(TaskPriority::None);
    let prioritized = task("priority", "Medium priority");
    let tasks = [absent, none, prioritized];
    assert_eq!(
        task_ids(&tasks, &unary_filters("priority", "is empty", None)),
        ["absent", "none"]
    );
    assert_eq!(
        task_ids(&tasks, &unary_filters("priority", "is not empty", None)),
        ["priority"]
    );
}

#[test]
fn optional_status_and_collection_can_be_filtered_by_presence() {
    let mut absent = task("absent", "Absent");
    absent.status = None;
    let mut present = task("present", "Present");
    present.remote_collection_id = Some(CollectionId::from("tasks"));
    let tasks = [absent, present];
    for column in ["status", "remotecollectionid"] {
        assert_eq!(
            task_ids(&tasks, &unary_filters(column, "is empty", None)),
            ["absent"]
        );
        assert_eq!(
            task_ids(&tasks, &unary_filters(column, "is not empty", None)),
            ["present"]
        );
    }
}

#[test]
fn event_optional_fields_use_the_same_empty_semantics_as_tasks() {
    let mut absent = event("absent", "Absent");
    absent.status = None;
    let mut empty = event("empty", "Empty");
    empty.status = None;
    empty.tags = Some(vec![]);
    empty.priority = Some(TaskPriority::None);
    let mut present = event("present", "Present");
    present.tags = Some(vec![Tag {
        id: "work".into(),
        name: "work".into(),
        color: None,
    }]);
    present.priority = Some(TaskPriority::Low);
    present.remote_collection_id = Some(CollectionId::from("events"));
    let events = [absent, empty, present];
    for column in ["tags", "priority", "status", "remotecollectionid"] {
        assert_eq!(
            event_ids(&events, &unary_filters(column, "is empty", None)),
            ["absent", "empty"],
            "{column}"
        );
        assert_eq!(
            event_ids(&events, &unary_filters(column, "is not empty", None)),
            ["present"],
            "{column}"
        );
    }
}

#[test]
fn empty_conditions_are_effective_or_branches_and_search_still_constrains_them() {
    let mut scheduled = task("scheduled", "Scheduled work");
    scheduled.start = Some("2026-11-20".into());
    let tasks = [
        task("empty", "Unscheduled work"),
        scheduled,
        task("other", "Other"),
    ];
    let filters = serde_json::from_value(json!({"operator": "or", "children": [
        {"kind": "condition", "filter": {"column": "start", "operator": "is empty"}},
        {"kind": "condition", "filter": {"column": "title", "operator": "is", "value": "Scheduled work"}},
        {"kind": "condition", "filter": {"column": "tags", "operator": "is", "value": []}}
    ]})).unwrap();
    let ids: Vec<_> = query_tasks(&tasks, &filters, &[], "work")
        .iter()
        .map(|task| task.id.as_str().to_owned())
        .collect();
    assert_eq!(ids, ["empty", "scheduled"]);

    let filters = serde_json::from_value(json!({"operator": "or", "children": [
        {"kind": "condition", "filter": {"column": "start", "operator": "is not empty"}},
        {"kind": "condition", "filter": {"column": null, "operator": "is empty"}}
    ]}))
    .unwrap();
    assert_eq!(task_ids(&tasks, &filters), ["scheduled"]);
}
