//! Task verbs: list, show, add, set, done, remove.

use chrono::{SecondsFormat, Utc};

use crate::error::AppError;
use crate::features::calendar::CalendarComponent;
use crate::features::calendar::{ops, query, Store};
use crate::features::calendar::{AppTask, AppTaskStatus, CollectionId, FilterGroup, TaskId};

use super::super::output::print_json;
use super::super::session::open;
use super::super::spec::{ListTasksArgs, TaskAddArgs, TaskFields, TaskSetArgs};
use super::{find_task, require_component, resolve_list};

/// # Errors
///
/// Propagates session, query and output failures.
pub async fn list_tasks(args: &ListTasksArgs) -> Result<(), AppError> {
    let session = open().await?;
    let search = args.search.as_deref().unwrap_or_default();
    let mut tasks =
        query::query_tasks(&session.store.tasks(), &FilterGroup::default(), &[], search);

    if let Some(status) = args.status {
        let wanted = AppTaskStatus::from(status);
        tasks.retain(|task| task.status.as_ref() == Some(&wanted));
    }
    if args.open {
        tasks.retain(|task| task.status != Some(AppTaskStatus::Done));
    }

    print_json(&tasks)
}

/// # Errors
///
/// Propagates session, id-resolution and output failures.
pub async fn show_task(ids: &[String]) -> Result<(), AppError> {
    let session = open().await?;
    let tasks = resolve_tasks(&session.store, ids)?;
    print_json(&tasks)
}

fn resolve_tasks(store: &Store, ids: &[String]) -> Result<Vec<AppTask>, AppError> {
    ids.iter().map(|id| find_task(store, id)).collect()
}

/// # Errors
///
/// Propagates session, option, list-resolution and write failures.
pub async fn add_task(args: &TaskAddArgs) -> Result<(), AppError> {
    let mut session = open().await?;
    let mut task = empty_task(uuid::Uuid::new_v4().to_string(), args.title.clone());
    apply_task_fields(&mut task, &args.fields);

    if let Some(list) = args.fields.list.as_deref() {
        let (href, component) = resolve_list(&session.store, list)?;
        require_component(component, CalendarComponent::VTodo, list)?;
        task.remote_collection_id = Some(CollectionId(href));
    }

    let created = ops::create_task(&mut session.store, &session.client, task).await?;
    print_json(&created)
}

/// # Errors
///
/// Propagates session, option, id/list-resolution, write and output failures.
pub async fn set_task(args: &TaskSetArgs) -> Result<(), AppError> {
    let mut session = open().await?;
    let mut updated = Vec::new();

    for id in &args.ids {
        let mut task = find_task(&session.store, id)?;
        apply_task_set(&mut task, args);

        if let Some(list) = args.fields.list.as_deref() {
            let (href, component) = resolve_list(&session.store, list)?;
            require_component(component, CalendarComponent::VTodo, list)?;
            task.remote_collection_id = Some(CollectionId(href));
        }

        ops::update_task(&mut session.store, &session.client, task.clone()).await?;
        updated.push(find_task(&session.store, task.id.as_str())?);
    }

    print_json(&updated)
}

/// # Errors
///
/// Propagates session, id-resolution, write and output failures.
pub async fn done_task(ids: &[String]) -> Result<(), AppError> {
    let mut session = open().await?;
    let mut updated = Vec::new();

    for id in ids {
        let mut task = find_task(&session.store, id)?;
        task.status = Some(AppTaskStatus::Done);
        task.completed_at = Some(Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true));

        ops::update_task(&mut session.store, &session.client, task.clone()).await?;
        updated.push(find_task(&session.store, task.id.as_str())?);
    }

    print_json(&updated)
}

/// # Errors
///
/// Propagates session, id-resolution, delete and output failures.
pub async fn remove_task(ids: &[String]) -> Result<(), AppError> {
    let mut session = open().await?;
    let mut deleted = Vec::new();

    for id in ids {
        let task = find_task(&session.store, id)?;
        ops::delete_task(&mut session.store, &session.client, task.id.as_str()).await?;
        deleted.push(serde_json::json!({ "deleted": task.id.as_str() }));
    }

    print_json(&deleted)
}

pub(crate) const fn empty_task(id: String, title: String) -> AppTask {
    AppTask {
        id: TaskId(id),
        title,
        status: Some(AppTaskStatus::Todo),
        priority: None,
        tags: None,
        parent_task: None,
        dependencies: None,
        est: None,
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

pub(crate) fn apply_task_fields(task: &mut AppTask, fields: &TaskFields) {
    if let Some(notes) = &fields.notes {
        task.notes = Some(notes.clone());
    }
    if let Some(due) = &fields.due {
        task.due = Some(due.clone());
    }
    if let Some(start) = &fields.start {
        task.start = Some(start.clone());
    }
    if let Some(priority) = fields.priority {
        task.priority = Some(priority.into());
    }
    if let Some(status) = fields.status {
        task.status = Some(status.into());
    }
    if let Some(tz) = &fields.tz {
        task.timezone = Some(tz.clone());
    }
}

pub(crate) fn apply_task_set(task: &mut AppTask, args: &TaskSetArgs) {
    if let Some(title) = &args.title {
        task.title.clone_from(title);
    }
    apply_task_fields(task, &args.fields);
    if args.clear_due {
        task.due = None;
    }
    if args.clear_start {
        task.start = None;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use clap::Parser;

    use super::*;
    use crate::cli::spec::{Cli, Command};
    use crate::features::calendar::{Resource, ResourceItems, TaskPriority};
    use crate::settings::AppSettings;

    fn parse_set(tokens: &[&str]) -> TaskSetArgs {
        let mut argv = vec!["taskroot", "task-set", "id"];
        argv.extend_from_slice(tokens);
        let parsed = Cli::try_parse_from(argv).unwrap();
        let Some(Command::TaskSet(set_args)) = parsed.command else {
            panic!("expected task-set");
        };
        set_args
    }

    #[test]
    fn flags_become_task_fields() {
        let mut task = empty_task("id".into(), "old title".into());
        let args = parse_set(&[
            "--title",
            "new title",
            "--notes",
            "a note",
            "--due",
            "2026-10-28",
            "--start",
            "2026-10-27",
            "--priority",
            "urgent",
            "--status",
            "next-up",
            "--tz",
            "Asia/Calcutta",
        ]);

        apply_task_set(&mut task, &args);

        assert_eq!(task.title, "new title");
        assert_eq!(task.notes.as_deref(), Some("a note"));
        assert_eq!(task.due.as_deref(), Some("2026-10-28"));
        assert_eq!(task.start.as_deref(), Some("2026-10-27"));
        assert_eq!(task.priority, Some(TaskPriority::Urgent));
        assert_eq!(task.status, Some(AppTaskStatus::NextUp));
        assert_eq!(task.timezone.as_deref(), Some("Asia/Calcutta"));
    }

    #[test]
    fn clearing_a_date_wins_even_when_a_new_date_is_given() {
        let mut task = empty_task("id".into(), "task".into());
        let args = parse_set(&["--due", "2026-10-28", "--clear-due"]);

        apply_task_set(&mut task, &args);

        assert_eq!(task.due, None);
    }

    #[test]
    fn show_task_resolves_multiple_ids_in_order() {
        let mut store = Store {
            data_dir: PathBuf::new(),
            resources: BTreeMap::new(),
            calendars: Vec::new(),
            settings: AppSettings::default(),
            ui_state: serde_json::Map::new(),
            revision: 0,
        };
        for (id, title) in [("a2a8-1111", "First"), ("b7c1-2222", "Second")] {
            store.resources.insert(
                format!("{id}.ics"),
                Resource {
                    href: format!("{id}.ics"),
                    collection: "/tasks/".to_string(),
                    etag: None,
                    raw: String::new(),
                    items: ResourceItems::Tasks(vec![empty_task(id.into(), title.into())]),
                },
            );
        }

        let found = resolve_tasks(&store, &["b7c1".to_string(), "a2a8".to_string()]).unwrap();
        let ids: Vec<&str> = found.iter().map(|task| task.id.as_str()).collect();
        assert_eq!(ids, vec!["b7c1-2222", "a2a8-1111"]);
    }
}
