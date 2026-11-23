use super::*;
use crate::features::calendar::domain::FilterGroup;
use serde_json::Value;

fn condition(column: &str, operator: &str, value: Value) -> Value {
    let fields = serde_json::Map::from_iter([
        ("column".into(), json!(column)),
        ("operator".into(), json!(operator)),
        ("value".into(), value),
    ]);
    json!({"kind": "condition", "filter": Value::Object(fields)})
}

fn group(operator: &str, children: Vec<Value>) -> Value {
    json!({"kind": "group", "operator": operator, "children": Value::Array(children)})
}

fn task_ids(operator: &str, children: Vec<Value>, search: &str) -> Vec<String> {
    let tasks = vec![task("a", "Alpha"), task("b", "Beta"), task("c", "Gamma")];
    let filters: FilterGroup<AppTaskFilter> =
        serde_json::from_value(json!({"operator": operator, "children": Value::Array(children)}))
            .unwrap();
    query_tasks(&tasks, &filters, &[], search)
        .iter()
        .map(|task| task.id.as_str().to_string())
        .collect()
}

fn event_ids(operator: &str, children: Vec<Value>, search: &str) -> Vec<String> {
    let events = vec![event("a", "Alpha"), event("b", "Beta"), event("c", "Gamma")];
    let filters: FilterGroup<AppEventFilter> =
        serde_json::from_value(json!({"operator": operator, "children": Value::Array(children)}))
            .unwrap();
    query_events(&events, &filters, &[], search)
        .iter()
        .map(|event| event.id.as_str().to_string())
        .collect()
}

fn title(value: &str) -> Value {
    condition("title", "is", json!(value))
}

#[test]
fn root_and_requires_both_conditions_while_or_accepts_either() {
    let children = vec![title("Alpha"), title("Beta")];
    assert_eq!(task_ids("and", children.clone(), ""), Vec::<String>::new());
    assert_eq!(task_ids("or", children.clone(), ""), ["a", "b"]);
    assert_eq!(event_ids("and", children.clone(), ""), Vec::<String>::new());
    assert_eq!(event_ids("or", children, ""), ["a", "b"]);
}

#[test]
fn nested_or_within_and_is_constrained_by_the_other_condition() {
    let children = vec![
        group("or", vec![title("Alpha"), title("Beta")]),
        condition("title", "contains", json!("ph")),
    ];
    assert_eq!(task_ids("and", children.clone(), ""), ["a"]);
    assert_eq!(event_ids("and", children, ""), ["a"]);
}

#[test]
fn nested_and_within_or_does_not_constrain_the_other_branch() {
    let children = vec![
        title("Alpha"),
        group(
            "and",
            vec![
                condition("title", "is", json!(["Beta", "Gamma"])),
                condition("title", "contains", json!("et")),
            ],
        ),
    ];
    assert_eq!(task_ids("or", children.clone(), ""), ["a", "b"]);
    assert_eq!(event_ids("or", children, ""), ["a", "b"]);
}

#[test]
fn multi_value_inclusions_match_any_and_exclusions_match_none() {
    for (operator, expected) in [
        ("is", vec!["a", "b"]),
        ("is not", vec!["c"]),
        ("contains", vec!["a", "b"]),
        ("does not contain", vec!["c"]),
    ] {
        let children = vec![group(
            "and",
            vec![condition("title", operator, json!(["Alpha", "Beta"]))],
        )];
        assert_eq!(task_ids("or", children.clone(), ""), expected);
        assert_eq!(event_ids("or", children, ""), expected);
    }
}

#[test]
fn unfinished_conditions_and_empty_nested_groups_are_ignored_under_or() {
    let unfinished = vec![
        json!({"kind": "condition", "filter": {}}),
        json!({"kind": "condition", "filter": {"column": null, "value": "Alpha"}}),
        json!({"kind": "condition", "filter": {"column": "title"}}),
        condition("title", "is", Value::Null),
        condition("title", "is", json!([])),
        condition("title", "contains", json!("")),
        condition("title", "does not contain", json!([null, ""])),
        group("and", vec![]),
        group(
            "or",
            vec![group("and", vec![condition("title", "is", Value::Null)])],
        ),
    ];
    for empty in unfinished {
        let children = vec![title("Alpha"), empty.clone()];
        assert_eq!(task_ids("or", children.clone(), ""), ["a"], "{empty}");
        assert_eq!(event_ids("or", children, ""), ["a"], "{empty}");
        assert_eq!(task_ids("or", vec![empty.clone()], ""), ["a", "b", "c"]);
        assert_eq!(event_ids("or", vec![empty], ""), ["a", "b", "c"]);
    }
    for operator in ["and", "or"] {
        assert_eq!(task_ids(operator, vec![], ""), ["a", "b", "c"]);
        assert_eq!(event_ids(operator, vec![], ""), ["a", "b", "c"]);
    }
}

#[test]
fn unfinished_array_entries_do_not_turn_a_completed_condition_into_match_all() {
    let children = vec![condition("title", "contains", json!([null, "", "Alpha"]))];
    assert_eq!(task_ids("or", children.clone(), ""), ["a"]);
    assert_eq!(event_ids("or", children, ""), ["a"]);
}

#[test]
fn deeper_mixed_groups_preserve_boolean_structure() {
    let children = vec![
        group(
            "and",
            vec![
                group(
                    "or",
                    vec![
                        group(
                            "and",
                            vec![title("Alpha"), condition("title", "contains", json!("ph"))],
                        ),
                        group("or", vec![title("Beta"), title("Gamma")]),
                    ],
                ),
                condition("title", "is not", json!(["Alpha", "Gamma"])),
            ],
        ),
        group(
            "and",
            vec![title("Gamma"), condition("title", "contains", json!("zz"))],
        ),
    ];
    assert_eq!(task_ids("or", children.clone(), ""), ["b"]);
    assert_eq!(event_ids("or", children, ""), ["b"]);
}

#[test]
fn search_is_anded_with_the_whole_tree() {
    let children = vec![title("Alpha"), title("Beta")];
    assert_eq!(task_ids("or", children.clone(), " ALP "), ["a"]);
    assert_eq!(event_ids("or", children.clone(), " ALP "), ["a"]);
    assert_eq!(
        task_ids("or", children.clone(), "Gamma"),
        Vec::<String>::new()
    );
    assert_eq!(event_ids("or", children, "Gamma"), Vec::<String>::new());
}

#[test]
fn sorting_is_applied_after_matching_recursive_filters() {
    let tasks = vec![task("a", "Alpha"), task("b", "Beta"), task("c", "Gamma")];
    let filters: FilterGroup<AppTaskFilter> = serde_json::from_value(json!({
        "operator": "or", "children": [title("Alpha"), title("Beta")]
    }))
    .unwrap();
    let sorted = query_tasks(
        &tasks,
        &filters,
        &[AppTaskSort {
            column: Some(AppTaskFilterColumn::Title),
            direction: Some(SortDirection::Desc),
        }],
        "",
    );
    assert_eq!(
        sorted
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>(),
        ["b", "a"]
    );
}

#[test]
fn missing_or_null_leaf_operator_defaults_to_is() {
    for filter in [
        json!({"column": "title", "value": "Alpha"}),
        json!({"column": "title", "operator": null, "value": "Alpha"}),
    ] {
        let children = vec![json!({"kind": "condition", "filter": filter})];
        assert_eq!(task_ids("or", children.clone(), ""), ["a"]);
        assert_eq!(event_ids("or", children, ""), ["a"]);
    }
}

#[test]
fn an_exclusion_branch_can_be_combined_with_an_inclusion_under_or() {
    let children = vec![
        condition("title", "is not", json!(["Alpha", "Beta"])),
        title("Alpha"),
    ];
    assert_eq!(task_ids("or", children.clone(), ""), ["a", "c"]);
    assert_eq!(event_ids("or", children, ""), ["a", "c"]);
}
