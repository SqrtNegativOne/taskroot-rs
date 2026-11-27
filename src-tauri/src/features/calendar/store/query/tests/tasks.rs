use super::super::columns::resolve_filter_date;
use super::*;
use chrono::NaiveDate;

#[test]
fn status_filter_includes_and_excludes() {
    let mut done = task("b", "Done");
    done.status = Some(crate::features::calendar::domain::AppTaskStatus::Done);
    let tasks = vec![task("a", "Open"), done];

    let open = query_tasks(
        &tasks,
        &FilterGroup::from(vec![filter(
            AppTaskFilterColumn::Status,
            FilterOperator::IsNot,
            json!(["done"]),
        )]),
        &[],
        "",
    );
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].id.as_str(), "a");
}

#[test]
fn tags_filter_matches_by_name() {
    let mut tagged = task("a", "Tagged");
    tagged.tags = Some(vec![Tag {
        id: "t".into(),
        name: "work".into(),
        color: None,
    }]);
    let tasks = vec![tagged, task("b", "Plain")];

    let matched = query_tasks(
        &tasks,
        &FilterGroup::from(vec![filter(
            AppTaskFilterColumn::Tags,
            FilterOperator::Is,
            json!(["work"]),
        )]),
        &[],
        "",
    );
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].id.as_str(), "a");
}

#[test]
fn title_sort_is_case_insensitive_and_ties_break_by_id() {
    let tasks = vec![task("b", "beta"), task("a", "Alpha"), task("c", "alpha")];
    let sorted = query_tasks(
        &tasks,
        &FilterGroup::default(),
        &[AppTaskSort {
            column: Some(AppTaskFilterColumn::Title),
            direction: Some(SortDirection::Asc),
        }],
        "",
    );
    let ids: Vec<&str> = sorted.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, vec!["a", "c", "b"]);
}

#[test]
fn created_sort_uses_added_timestamp_not_id() {
    let mut older = task("z-older", "Older");
    older.added = Some("2024-01-01T00:00:00Z".parse().unwrap());
    let mut newer = task("a-newer", "Newer");
    newer.added = Some("2025-01-01T00:00:00Z".parse().unwrap());
    let tasks = vec![newer, older];

    let sorted = query_tasks(
        &tasks,
        &FilterGroup::default(),
        &[AppTaskSort {
            column: Some(AppTaskFilterColumn::Added),
            direction: Some(SortDirection::Asc),
        }],
        "",
    );
    let ids: Vec<&str> = sorted.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, vec!["z-older", "a-newer"]);
}

#[test]
fn search_matches_title_and_tag_names() {
    let mut tagged = task("a", "Something");
    tagged.tags = Some(vec![Tag {
        id: "t".into(),
        name: "errand".into(),
        color: None,
    }]);
    let tasks = vec![tagged, task("b", "Other")];

    let matched = query_tasks(&tasks, &FilterGroup::default(), &[], "errand");
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].id.as_str(), "a");
}

#[test]
fn start_date_filters_compare_calendar_days() {
    let mut early = task("a", "Early");
    early.start = Some("2026-09-07".parse().unwrap());
    let mut late = task("b", "Late");
    late.start = Some("2026-09-10T09:00:00".parse().unwrap());
    let tasks = vec![early, late, task("c", "No start")];

    let before = query_tasks(
        &tasks,
        &FilterGroup::from(vec![filter(
            AppTaskFilterColumn::Start,
            FilterOperator::OnOrBefore,
            json!("2026-09-08"),
        )]),
        &[],
        "",
    );
    let ids: Vec<&str> = before.iter().map(|task| task.id.as_str()).collect();
    assert_eq!(ids, vec!["a"]);

    let after = query_tasks(
        &tasks,
        &FilterGroup::from(vec![filter(
            AppTaskFilterColumn::Start,
            FilterOperator::After,
            json!("2026-09-08"),
        )]),
        &[],
        "",
    );
    let ids: Vec<&str> = after.iter().map(|task| task.id.as_str()).collect();
    assert_eq!(ids, vec!["b"]);

    let on_day = query_tasks(
        &tasks,
        &FilterGroup::from(vec![filter(
            AppTaskFilterColumn::Start,
            FilterOperator::Is,
            json!("2026-09-10"),
        )]),
        &[],
        "",
    );
    let ids: Vec<&str> = on_day.iter().map(|task| task.id.as_str()).collect();
    assert_eq!(ids, vec!["b"]);
}

#[test]
fn date_filter_tokens_resolve_relative_to_today() {
    let today = NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();

    assert_eq!(resolve_filter_date("today", today), Some(today));
    assert_eq!(
        resolve_filter_date("tomorrow", today),
        NaiveDate::from_ymd_opt(2026, 11, 21)
    );
    assert_eq!(
        resolve_filter_date("yesterday", today),
        NaiveDate::from_ymd_opt(2026, 11, 19)
    );
    assert_eq!(
        resolve_filter_date("2026-11-20T09:00:00", today),
        Some(today)
    );
}
