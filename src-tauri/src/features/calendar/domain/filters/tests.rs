use super::*;
use crate::features::calendar::domain::{AppEventFilter, AppTaskFilter};
use serde_json::json;

#[test]
fn task_tree_json_roundtrip_preserves_tagged_nested_conditions() {
    let value = json!({
        "operator": "or",
        "children": [
            {"kind": "condition", "filter": {"column": "title", "operator": "contains", "value": "work"}},
            {"kind": "group", "operator": "and", "children": [
                {"kind": "condition", "filter": {"column": "priority", "operator": "is", "value": [3, 4]}}
            ]}
        ]
    });
    let filters: FilterGroup<AppTaskFilter> = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(filters).unwrap(), value);
}

#[test]
fn event_tree_json_roundtrip_preserves_tagged_nested_conditions() {
    let value = json!({
        "operator": "and",
        "children": [
            {"kind": "group", "operator": "or", "children": [
                {"kind": "condition", "filter": {"column": "status", "operator": "is not", "value": ["cancelled"]}}
            ]}
        ]
    });
    let filters: FilterGroup<AppEventFilter> = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(filters).unwrap(), value);
}

#[test]
fn trees_reject_untagged_unknown_and_malformed_nodes() {
    for node in [
        json!({"filter": {"column": "title", "value": "work"}}),
        json!({"operator": "or", "children": []}),
        json!({"kind": "unknown", "filter": {}}),
        json!({"kind": "condition"}),
        json!({"kind": "condition", "filter": null}),
        json!({"kind": "group", "children": []}),
        json!({"kind": "group", "operator": "xor", "children": []}),
        json!({"kind": "group", "operator": "or", "children": null}),
        json!({"kind": "Group", "operator": "or", "children": []}),
    ] {
        let value = json!({"operator": "and", "children": [node]});
        assert!(
            serde_json::from_value::<FilterGroup<AppTaskFilter>>(value.clone()).is_err(),
            "{value}"
        );
        assert!(
            serde_json::from_value::<FilterGroup<AppEventFilter>>(value.clone()).is_err(),
            "{value}"
        );
    }
    for value in [
        json!([]),
        json!({"operator": "AND", "children": []}),
        json!({"operator": "or"}),
        json!({"children": []}),
    ] {
        assert!(
            serde_json::from_value::<FilterGroup<AppTaskFilter>>(value.clone()).is_err(),
            "{value}"
        );
    }
}

#[test]
fn default_and_flat_constructor_produce_and_groups() {
    assert_eq!(LogicalOperator::default(), LogicalOperator::And);
    let empty: FilterGroup<AppTaskFilter> = FilterGroup::default();
    assert_eq!(
        serde_json::to_value(empty).unwrap(),
        json!({"operator": "and", "children": []})
    );
    let leaf: AppTaskFilter = serde_json::from_value(json!({
        "column": "title", "operator": "is", "value": "work"
    }))
    .unwrap();
    assert_eq!(
        serde_json::to_value(FilterGroup::from(vec![leaf])).unwrap(),
        json!({
            "operator": "and", "children": [
                {"kind": "condition", "filter": {"column": "title", "operator": "is", "value": "work"}}
            ]
        })
    );
}
