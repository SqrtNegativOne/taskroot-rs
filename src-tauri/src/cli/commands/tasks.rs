//! Task verbs: list, show, add, set, done, remove.

use chrono::{SecondsFormat, Utc};

use crate::caldav::model::CalendarComponent;
use crate::domain::{
    AppTask, AppTaskStatus, CollectionId, TaskId, TaskPriority,
};
use crate::error::AppError;
use crate::store::{ops, query};

use super::super::args::Cli;
use super::super::output::print_json;
use super::super::session::open;
use super::{find_task, require_component, resolve_list};

/// # Errors
///
/// Propagates session, query and output failures.
pub async fn list_tasks(cli: &Cli) -> Result<(), AppError> {
    let session = open().await?;
    let search = cli.option("search").unwrap_or_default();
    let mut tasks = query::query_tasks(&session.store.tasks(), &[], &[], search);

    if let Some(status) = cli.option("status") {
        let wanted = parse_task_status(status)?;
        tasks.retain(|task| task.status.as_ref() == Some(&wanted));
    }
    if cli.has("open") {
        tasks.retain(|task| task.status != Some(AppTaskStatus::Done));
    }

    print_json(&tasks)
}

/// # Errors
///
/// Propagates session, id-resolution and output failures.
pub async fn show_task(cli: &Cli) -> Result<(), AppError> {
    let session = open().await?;
    let task = find_task(&session.store, cli.require_positional(1, "<id>")?)?;
    print_json(&task)
}

/// # Errors
///
/// Propagates session, option, list-resolution and write failures.
pub async fn add_task(cli: &Cli) -> Result<(), AppError> {
    let mut session = open().await?;
    let title = cli.require_option("title")?.to_string();
    let mut task = empty_task(uuid::Uuid::new_v4().to_string(), title);
    apply_task_options(&mut task, cli)?;

    if let Some(list) = cli.option("list") {
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
pub async fn set_task(cli: &Cli) -> Result<(), AppError> {
    let mut session = open().await?;
    let ids = cli.require_all_positionals(1, "<id>")?;
    let mut updated = Vec::new();

    for id in ids {
        let mut task = find_task(&session.store, id)?;
        apply_task_options(&mut task, cli)?;

        if let Some(list) = cli.option("list") {
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
pub async fn done_task(cli: &Cli) -> Result<(), AppError> {
    let mut session = open().await?;
    let mut updated = Vec::new();

    for id in cli.require_all_positionals(1, "<id>")? {
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
pub async fn remove_task(cli: &Cli) -> Result<(), AppError> {
    let mut session = open().await?;
    let mut deleted = Vec::new();

    for id in cli.require_all_positionals(1, "<id>")? {
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
        canvas_x: None,
        canvas_y: None,
        on_canvas: None,
        remote_id: None,
        remote_collection_id: None,
        notes: None,
        tabs: None,
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

pub(crate) fn apply_task_options(task: &mut AppTask, cli: &Cli) -> Result<(), AppError> {
    if let Some(title) = cli.option("title") {
        task.title = title.to_string();
    }
    if let Some(notes) = cli.option("notes") {
        task.notes = Some(notes.to_string());
    }
    if let Some(due) = cli.option("due") {
        task.due = Some(due.to_string());
    }
    if cli.has("clear-due") {
        task.due = None;
    }
    if let Some(start) = cli.option("start") {
        task.start = Some(start.to_string());
    }
    if cli.has("clear-start") {
        task.start = None;
    }
    if let Some(priority) = cli.option("priority") {
        task.priority = Some(parse_priority(priority)?);
    }
    if let Some(status) = cli.option("status") {
        task.status = Some(parse_task_status(status)?);
    }
    if let Some(tz) = cli.option("tz") {
        task.timezone = Some(tz.to_string());
    }
    Ok(())
}

fn parse_task_status(value: &str) -> Result<AppTaskStatus, AppError> {
    match value {
        "todo" => Ok(AppTaskStatus::Todo),
        "next-up" | "nextup" | "next" => Ok(AppTaskStatus::NextUp),
        "doing" | "in-progress" => Ok(AppTaskStatus::Doing),
        "done" | "completed" => Ok(AppTaskStatus::Done),
        other => Err(AppError::InvalidInput(format!(
            "unknown status `{other}` (todo, next-up, doing, done)"
        ))),
    }
}

fn parse_priority(value: &str) -> Result<TaskPriority, AppError> {
    match value {
        "0" | "none" => Ok(TaskPriority::None),
        "1" | "low" => Ok(TaskPriority::Low),
        "2" | "medium" => Ok(TaskPriority::Medium),
        "3" | "high" => Ok(TaskPriority::High),
        "4" | "urgent" => Ok(TaskPriority::Urgent),
        other => Err(AppError::InvalidInput(format!(
            "unknown priority `{other}` (0-4 or low/medium/high/urgent)"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::Cli;

    fn cli(tokens: &[&str]) -> Cli {
        Cli::parse(tokens.iter().map(|token| (*token).to_string()))
    }

    #[test]
    fn flags_become_task_fields() {
        let mut task = empty_task("id".into(), "old title".into());
        let parsed = cli(&[
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
            "doing",
            "--tz",
            "Asia/Calcutta",
        ]);

        apply_task_options(&mut task, &parsed).unwrap();

        assert_eq!(task.title, "new title");
        assert_eq!(task.notes.as_deref(), Some("a note"));
        assert_eq!(task.due.as_deref(), Some("2026-10-28"));
        assert_eq!(task.start.as_deref(), Some("2026-10-27"));
        assert_eq!(task.priority, Some(TaskPriority::Urgent));
        assert_eq!(task.status, Some(AppTaskStatus::Doing));
        assert_eq!(task.timezone.as_deref(), Some("Asia/Calcutta"));
    }

    #[test]
    fn clearing_a_date_wins_even_when_a_new_date_is_given() {
        let mut task = empty_task("id".into(), "task".into());
        let parsed = cli(&["--due", "2026-10-28", "--clear-due"]);

        apply_task_options(&mut task, &parsed).unwrap();

        assert_eq!(task.due, None);
    }

    #[test]
    fn an_unknown_priority_or_status_is_rejected() {
        let mut task = empty_task("id".into(), "task".into());

        let priority = apply_task_options(&mut task, &cli(&["--priority", "9"]));
        let status = apply_task_options(&mut task, &cli(&["--status", "later"]));

        assert!(matches!(priority, Err(AppError::InvalidInput(_))));
        assert!(matches!(status, Err(AppError::InvalidInput(_))));
    }
}
