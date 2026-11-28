//! Task verbs: list, show, add, set, done, remove.

use chrono::{SubsecRound, Utc};

use crate::error::AppError;
use crate::features::calendar::CalendarComponent;
use crate::features::calendar::{
    ops, query, AppTask, AppTaskStatus, CollectionId, FilterGroup, Store, TaskId, TaskPatch,
};

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
    let stored = session.store.read(Store::tasks).await;
    let mut tasks = query::query_tasks(&stored, &FilterGroup::default(), &[], search);

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
    let tasks = session.store.read(|store| resolve_tasks(store, ids)).await?;
    print_json(&tasks)
}

fn resolve_tasks(store: &Store, ids: &[String]) -> Result<Vec<AppTask>, AppError> {
    ids.iter().map(|id| find_task(store, id)).collect()
}

/// # Errors
///
/// Propagates session, option, list-resolution and write failures.
pub async fn add_task(args: &TaskAddArgs) -> Result<(), AppError> {
    let session = open().await?;
    let mut task = empty_task(uuid::Uuid::new_v4().to_string(), args.title.clone());
    apply_task_fields(&mut task, &args.fields);

    if let Some(list) = args.fields.list.as_deref() {
        let (href, component) = session.store.read(|store| resolve_list(store, list)).await?;
        require_component(component, CalendarComponent::VTodo, list)?;
        task.remote_collection_id = Some(CollectionId(href));
    }

    let created = ops::create_task(&session.store, &session.client, task).await?;
    print_json(&created)
}

/// # Errors
///
/// Propagates session, option, id/list-resolution, write and output failures.
pub async fn set_task(args: &TaskSetArgs) -> Result<(), AppError> {
    let session = open().await?;
    let list_href = if let Some(list) = args.fields.list.as_deref() {
        let (href, component) = session.store.read(|store| resolve_list(store, list)).await?;
        require_component(component, CalendarComponent::VTodo, list)?;
        Some(CollectionId(href))
    } else {
        None
    };
    let patches = task_set_patches(args, list_href);
    let mut updated = Vec::new();

    for id in &args.ids {
        let task_id = session.store.read(|store| find_task(store, id)).await?.id;
        ops::update_task(&session.store, &session.client, task_id.as_str(), &patches).await?;
        updated.push(
            session
                .store
                .read(|store| find_task(store, task_id.as_str()))
                .await?,
        );
    }

    print_json(&updated)
}

/// # Errors
///
/// Propagates session, id-resolution, write and output failures.
pub async fn done_task(ids: &[String]) -> Result<(), AppError> {
    let session = open().await?;
    let patches = [
        TaskPatch::Status(Some(AppTaskStatus::Done)),
        TaskPatch::CompletedAt(Some(Utc::now().trunc_subsecs(0))),
    ];
    let mut updated = Vec::new();

    for id in ids {
        let task_id = session.store.read(|store| find_task(store, id)).await?.id;
        ops::update_task(&session.store, &session.client, task_id.as_str(), &patches).await?;
        updated.push(
            session
                .store
                .read(|store| find_task(store, task_id.as_str()))
                .await?,
        );
    }

    print_json(&updated)
}

/// # Errors
///
/// Propagates session, id-resolution, delete and output failures.
pub async fn remove_task(ids: &[String]) -> Result<(), AppError> {
    let session = open().await?;
    let mut deleted = Vec::new();

    for id in ids {
        let task_id = session.store.read(|store| find_task(store, id)).await?.id;
        ops::delete_task(&session.store, &session.client, task_id.as_str()).await?;
        deleted.push(serde_json::json!({ "deleted": task_id.as_str() }));
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
        time_spent: None,
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
    if let Some(due) = fields.due {
        task.due = Some(due);
    }
    if let Some(start) = fields.start {
        task.start = Some(start);
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

pub(crate) fn task_set_patches(
    args: &TaskSetArgs,
    list_href: Option<CollectionId>,
) -> Vec<TaskPatch> {
    let mut patches = Vec::new();
    if let Some(title) = &args.title {
        patches.push(TaskPatch::Title(title.clone()));
    }
    if let Some(notes) = &args.fields.notes {
        patches.push(TaskPatch::Notes(Some(notes.clone())));
    }
    if args.clear_due {
        patches.push(TaskPatch::Due(None));
    } else if let Some(due) = args.fields.due {
        patches.push(TaskPatch::Due(Some(due)));
    }
    if args.clear_start {
        patches.push(TaskPatch::Start(None));
    } else if let Some(start) = args.fields.start {
        patches.push(TaskPatch::Start(Some(start)));
    }
    if let Some(priority) = args.fields.priority {
        patches.push(TaskPatch::Priority(Some(priority.into())));
    }
    if let Some(status) = args.fields.status {
        patches.push(TaskPatch::Status(Some(status.into())));
    }
    if let Some(tz) = &args.fields.tz {
        patches.push(TaskPatch::Timezone(Some(tz.clone())));
    }
    if let Some(href) = list_href {
        patches.push(TaskPatch::RemoteCollectionId(Some(href)));
    }
    patches
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use clap::Parser;

    use super::*;
    use crate::cli::spec::{Cli, Command};
    use crate::features::calendar::{
        apply_task_patches, Resource, ResourceItems, TaskPriority,
    };
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

        let patches = task_set_patches(&args, None);
        apply_task_patches(&mut task, &patches);

        assert_eq!(task.title, "new title");
        assert_eq!(task.notes.as_deref(), Some("a note"));
        assert_eq!(task.due, Some("2026-10-28".parse().unwrap()));
        assert_eq!(task.start, Some("2026-10-27".parse().unwrap()));
        assert_eq!(task.priority, Some(TaskPriority::Urgent));
        assert_eq!(task.status, Some(AppTaskStatus::NextUp));
        assert_eq!(task.timezone.as_deref(), Some("Asia/Calcutta"));
    }

    #[test]
    fn clearing_a_date_wins_even_when_a_new_date_is_given() {
        let mut task = empty_task("id".into(), "task".into());
        let args = parse_set(&["--due", "2026-10-28", "--clear-due"]);

        let patches = task_set_patches(&args, None);
        apply_task_patches(&mut task, &patches);

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
