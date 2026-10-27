//! Headless Taskroot CLI for agents and scripts.
//!
//! It is a *client of the app's own domain layer*: it loads `account.json`, the
//! keychain password and every collection through `store::ops::full_load`, then
//! reuses `store::ops` / `store::query` / `screens::plan` for reads and writes.
//! Nothing here parses or serialises `.ics` itself, so the line-preserving write
//! path (invariants I1–I3) still owns every resource.
//!
//! Output is JSON on stdout; failures are `AppError` text on stderr with a
//! non-zero exit code.


use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use chrono::{Duration, Local, SecondsFormat, Utc};
use serde::Serialize;

use tauri_app_lib::caldav::account;
use tauri_app_lib::caldav::calendars;
use tauri_app_lib::caldav::client::CalDavClient;
use tauri_app_lib::caldav::discovery;
use tauri_app_lib::caldav::model::CalendarComponent;
use tauri_app_lib::caldav::transport::HttpClient;
use tauri_app_lib::domain::{
    AppEvent, AppTask, AppTaskStatus, CollectionId, EventId, EventStatus, TaskId, TaskPriority,
};
use tauri_app_lib::error::AppError;
use tauri_app_lib::screens::plan::expand_event_instances;
use tauri_app_lib::store::{ops, query, Store};

const APP_IDENTIFIER: &str = "com.arkma.tauri-app";

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), AppError> {
    let cli = Cli::from_env();
    let Some(command) = cli.positional(0) else {
        print_help();
        return Ok(());
    };

    match command {
        "help" => {
            print_help();
            Ok(())
        }
        "lists" => list_calendars().await,
        "list-rm" => remove_list(&cli).await,
        "tasks" => list_tasks(&cli).await,
        "task-show" => show_task(&cli).await,
        "task-add" => add_task(&cli).await,
        "task-set" => set_task(&cli).await,
        "task-done" => done_task(&cli).await,
        "task-rm" => remove_task(&cli).await,
        "events" => list_events(&cli).await,
        "event-add" => add_event(&cli).await,
        "event-rm" => remove_event(&cli).await,
        other => Err(AppError::InvalidInput(format!(
            "unknown command `{other}`; run `taskroot help`"
        ))),
    }
}

// ---------------------------------------------------------------- commands

async fn list_calendars() -> Result<(), AppError> {
    let session = open().await?;
    print_json(&session.store.calendars)
}

async fn remove_list(cli: &Cli) -> Result<(), AppError> {
    let mut session = open().await?;
    let needle = cli.require_positional(1, "<list>")?;
    let (href, _component) = resolve_list(&session.store, needle)?;

    calendars::delete(&session.dir, &session.client, &mut session.store, &href).await?;
    print_json(&serde_json::json!({ "deleted": href }))
}

async fn list_tasks(cli: &Cli) -> Result<(), AppError> {
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

async fn show_task(cli: &Cli) -> Result<(), AppError> {
    let session = open().await?;
    let task = find_task(&session.store, cli.require_positional(1, "<id>")?)?;
    print_json(&task)
}

async fn add_task(cli: &Cli) -> Result<(), AppError> {
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

async fn set_task(cli: &Cli) -> Result<(), AppError> {
    let mut session = open().await?;
    let ids = require_ids(cli, "<id>")?;
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

async fn done_task(cli: &Cli) -> Result<(), AppError> {
    let mut session = open().await?;
    let mut updated = Vec::new();

    for id in require_ids(cli, "<id>")? {
        let mut task = find_task(&session.store, id)?;
        task.status = Some(AppTaskStatus::Done);
        task.completed_at = Some(Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true));

        ops::update_task(&mut session.store, &session.client, task.clone()).await?;
        updated.push(find_task(&session.store, task.id.as_str())?);
    }

    print_json(&updated)
}

async fn remove_task(cli: &Cli) -> Result<(), AppError> {
    let mut session = open().await?;
    let mut deleted = Vec::new();

    for id in require_ids(cli, "<id>")? {
        let task = find_task(&session.store, id)?;
        ops::delete_task(&mut session.store, &session.client, task.id.as_str()).await?;
        deleted.push(serde_json::json!({ "deleted": task.id.as_str() }));
    }

    print_json(&deleted)
}

async fn list_events(cli: &Cli) -> Result<(), AppError> {
    let session = open().await?;
    let search = cli.option("search").unwrap_or_default();
    let events = query::query_events(&session.store.events(), &[], search);
    let start = cli
        .option("start")
        .map_or_else(today, ToString::to_string);
    let end = cli
        .option("end")
        .map_or_else(|| plus_days(14), ToString::to_string);
    print_json(&expand_event_instances(&events, &start, &end))
}

async fn add_event(cli: &Cli) -> Result<(), AppError> {
    let mut session = open().await?;
    let all_day = cli.has("all-day");
    let mut event = AppEvent {
        id: EventId(uuid::Uuid::new_v4().to_string()),
        remote_id: None,
        remote_collection_id: None,
        task_id: None,
        title: cli.require_option("title")?.to_string(),
        description: cli.option("notes").map(ToString::to_string),
        start_time: cli.require_option("start")?.to_string(),
        end_time: cli.require_option("end")?.to_string(),
        rrule: cli.option("rrule").map(ToString::to_string),
        exdates: None,
        recurring_event_id: None,
        original_start_time: None,
        status: Some(EventStatus::Confirmed),
        updated_at: None,
        color: None,
        etag: None,
        is_all_day: Some(all_day),
        timezone: cli.option("tz").map(ToString::to_string),
    };

    if let Some(list) = cli.option("list") {
        let (href, component) = resolve_list(&session.store, list)?;
        require_component(component, CalendarComponent::VEvent, list)?;
        event.remote_collection_id = Some(CollectionId(href));
    }

    let created = ops::create_event(&mut session.store, &session.client, event).await?;
    print_json(&created)
}

async fn remove_event(cli: &Cli) -> Result<(), AppError> {
    let mut session = open().await?;
    let id = cli.require_positional(1, "<id>")?;
    ops::delete_event(&mut session.store, &session.client, id).await?;
    print_json(&serde_json::json!({ "deleted": id }))
}

// ---------------------------------------------------------------- session

struct Session {
    dir: PathBuf,
    store: Store,
    client: CalDavClient,
}

/// Load the account, the keychain password and every collection.
async fn open() -> Result<Session, AppError> {
    let dir = data_dir()?;
    let mut store = Store::load(dir.clone())?;
    let mut account = account::read_account(&dir)?
        .ok_or_else(|| AppError::Auth("no CalDAV account is configured".to_string()))?;
    let password = match account::load_password(&account.username)? {
        Some(password) => password,
        None => std::env::var("TASKROOT_CALDAV_PASSWORD").map_err(|_| {
            AppError::Auth(
                "the CalDAV password is not in the OS keychain; set TASKROOT_CALDAV_PASSWORD"
                    .to_string(),
            )
        })?,
    };

    let http = HttpClient::new()?;
    let client = CalDavClient::from_account(&http, &account, &password)?;
    let calendars = discovery::refresh_collections(&dir, &client, &mut account).await;
    store.calendars.clone_from(&calendars);
    ops::full_load(&mut store, &client).await?;
    Ok(Session {
        dir,
        store,
        client,
    })
}

fn data_dir() -> Result<PathBuf, AppError> {
    if let Some(dir) = std::env::var_os("TASKROOT_DATA_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
        })
        .ok_or_else(|| {
            AppError::Store("cannot locate the app data directory; set TASKROOT_DATA_DIR".to_string())
        })?;
    Ok(base.join(APP_IDENTIFIER))
}

// ---------------------------------------------------------------- helpers

fn find_task(store: &Store, needle: &str) -> Result<AppTask, AppError> {
    let mut matches: Vec<AppTask> = store
        .tasks()
        .into_iter()
        .filter(|task| task.id.as_str().starts_with(needle))
        .collect();
    match matches.len() {
        0 => Err(AppError::NotFound(format!("no task id starts with `{needle}`"))),
        1 => Ok(matches.remove(0)),
        count => Err(AppError::InvalidInput(format!(
            "`{needle}` matches {count} tasks; use more of the id"
        ))),
    }
}

/// Every positional after the verb, so one invocation can act on a whole set.
fn require_ids<'a>(cli: &'a Cli, name: &str) -> Result<Vec<&'a str>, AppError> {
    let ids: Vec<&str> = cli.positionals.iter().skip(1).map(String::as_str).collect();
    if ids.is_empty() {
        return Err(AppError::InvalidInput(format!("missing argument {name}")));
    }
    Ok(ids)
}

/// Find a list by display name or href, of any component kind.
///
/// The caller decides which component it needs, so a task list passed to an
/// event verb fails with a message that names the mismatch instead of "not found".
fn resolve_list(store: &Store, needle: &str) -> Result<(String, CalendarComponent), AppError> {
    let found = store.calendars.iter().find(|calendar| {
        calendar.display_name.eq_ignore_ascii_case(needle) || calendar.href.starts_with(needle)
    });
    found
        .map(|calendar| (calendar.href.clone(), calendar.component))
        .ok_or_else(|| AppError::NotFound(format!("no list named `{needle}`")))
}

fn require_component(
    found: CalendarComponent,
    wanted: CalendarComponent,
    list: &str,
) -> Result<(), AppError> {
    if found == wanted {
        return Ok(());
    }
    Err(AppError::InvalidInput(format!(
        "`{list}` holds {found} items; this command needs a {wanted} list"
    )))
}

const fn empty_task(id: String, title: String) -> AppTask {
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

fn apply_task_options(task: &mut AppTask, cli: &Cli) -> Result<(), AppError> {
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

fn print_json<T: Serialize>(value: &T) -> Result<(), AppError> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| AppError::Internal(format!("could not encode JSON: {error}")))?;
    println!("{text}");
    Ok(())
}

fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn plus_days(days: i64) -> String {
    let later = Local::now()
        .checked_add_signed(Duration::days(days))
        .unwrap_or_else(Local::now);
    later.format("%Y-%m-%d").to_string()
}

// ---------------------------------------------------------------- argument parsing

/// `--key value` pairs plus bare positionals.
///
/// A `--key` with no following value (or a following `--other`) is stored as
/// `"true"`, which is how boolean flags are expressed.
struct Cli {
    positionals: Vec<String>,
    options: BTreeMap<String, String>,
}

impl Cli {
    fn from_env() -> Self {
        Self::parse(std::env::args().skip(1))
    }

    fn parse(tokens: impl IntoIterator<Item = String>) -> Self {
        let mut positionals = Vec::new();
        let mut options = BTreeMap::new();
        let mut tokens = tokens.into_iter().peekable();

        while let Some(token) = tokens.next() {
            let Some(key) = token.strip_prefix("--") else {
                positionals.push(token);
                continue;
            };
            let value = match tokens.peek() {
                Some(next) if !next.starts_with("--") => tokens.next().unwrap_or_default(),
                _ => "true".to_string(),
            };
            options.insert(key.to_string(), value);
        }

        Self {
            positionals,
            options,
        }
    }

    fn positional(&self, index: usize) -> Option<&str> {
        self.positionals.get(index).map(String::as_str)
    }

    fn require_positional(&self, index: usize, name: &str) -> Result<&str, AppError> {
        self.positional(index)
            .ok_or_else(|| AppError::InvalidInput(format!("missing argument {name}")))
    }

    fn option(&self, key: &str) -> Option<&str> {
        self.options.get(key).map(String::as_str)
    }

    fn require_option(&self, key: &str) -> Result<&str, AppError> {
        self.option(key)
            .ok_or_else(|| AppError::InvalidInput(format!("missing --{key} <value>")))
    }

    fn has(&self, key: &str) -> bool {
        self.options.contains_key(key)
    }
}

/// One CLI verb. `print_help` and `docs/cli.md` are both rendered from [`VERBS`],
/// so the help text cannot describe a command the dispatch does not have (the
/// reverse is covered by `every_dispatched_verb_is_documented`).
struct Verb {
    usage: &'static str,
    summary: &'static str,
}

const VERBS: &[Verb] = &[
    Verb { usage: "help", summary: "this text" },
    Verb { usage: "lists", summary: "calendars this account can see" },
    Verb { usage: "list-rm <list>", summary: "delete a list and everything in it" },
    Verb { usage: "tasks [--search S] [--status S] [--open]", summary: "list tasks" },
    Verb { usage: "task-show <id>...", summary: "print tasks exactly as stored" },
    Verb { usage: "task-add --title T [options]", summary: "create a task" },
    Verb { usage: "task-set <id>... [options]", summary: "change tasks; --list moves them" },
    Verb { usage: "task-done <id>...", summary: "mark tasks done" },
    Verb { usage: "task-rm <id>...", summary: "delete tasks" },
    Verb {
        usage: "events [--start D] [--end D] [--search S]",
        summary: "event instances in a date range (default: today + 14 days)",
    },
    Verb {
        usage: "event-add --title T --start S --end E [options]",
        summary: "create an event",
    },
    Verb { usage: "event-rm <id>", summary: "delete an event" },
];

const OPTIONS: &[(&str, &str)] = &[
    ("--title T", "title (required for the add verbs)"),
    ("--notes N", "description"),
    ("--due D", "task due date"),
    ("--start D, --end E", "task start date; event start and end"),
    ("--priority P", "0-4, or none/low/medium/high/urgent"),
    ("--status S", "todo, next-up, doing or done"),
    ("--tz ZONE", "IANA zone for timed values"),
    ("--list NAME", "target list; on task-set this moves the task"),
    ("--clear-due, --clear-start", "drop a date"),
    ("--all-day", "event has no time of day"),
    ("--rrule R", "recurrence rule"),
    ("--search S", "substring filter"),
    ("--open", "only tasks that are not done"),
];

const HELP_TAIL: &str = "\
An id may be a unique prefix. Verbs that take <id>... accept several ids and do
one load for all of them; they print a JSON array of results. Dates are
YYYY-MM-DD for all-day values, YYYY-MM-DDTHH:MM:SS wall clock for timed ones.

env: TASKROOT_DATA_DIR, TASKROOT_CALDAV_PASSWORD (keychain fallback)

Reads and writes go through Radicale via Taskroot's own store, so every edit is
a line-preserving merge into the fetched .ics resource.";

fn help_text() -> String {
    let width = VERBS.iter().map(|verb| verb.usage.len()).max().unwrap_or(0);
    let mut text = String::from("taskroot — headless Taskroot CLI (JSON on stdout)\n\n");
    for verb in VERBS {
        let _ = writeln!(text, "  {:<width$}  {}", verb.usage, verb.summary);
    }
    text.push_str("\noptions:\n");
    for (flag, summary) in OPTIONS {
        let _ = writeln!(text, "  {flag} — {summary}");
    }
    text.push('\n');
    text.push_str(HELP_TAIL);
    text.push('\n');
    text
}

/// The `docs/cli.md` rendering, written by the `exports_the_cli_reference` test.
#[cfg(test)]
fn help_markdown() -> String {
    let mut text = String::from(
        "# taskroot CLI\n\n\
         <!-- Generated from the VERBS and OPTIONS tables in src-tauri/src/bin/taskroot.rs \
         by `cargo nextest run`. Do not edit. -->\n\n\
         Headless Taskroot for agents and scripts: JSON on stdout, `AppError` text on\n\
         stderr with a non-zero exit code.\n\n\
         ```\nbun run cli -- <verb>\n```\n\n\
         ## Commands\n\n| Command | Does |\n| --- | --- |\n",
    );
    for verb in VERBS {
        let _ = writeln!(text, "| `{}` | {} |", verb.usage, verb.summary);
    }
    text.push_str("\n## Options\n\n| Flag | Meaning |\n| --- | --- |\n");
    for (flag, summary) in OPTIONS {
        let _ = writeln!(text, "| `{flag}` | {summary} |");
    }
    text.push('\n');
    text.push_str(HELP_TAIL);
    text.push('\n');
    text
}

fn print_help() {
    print!("{}", help_text());
}

#[cfg(test)]
mod tests {

    use super::*;
    use tauri_app_lib::caldav::model::CalendarInfo;
    use tauri_app_lib::settings::AppSettings;
    use tauri_app_lib::store::{Resource, ResourceItems};

    fn cli(tokens: &[&str]) -> Cli {
        Cli::parse(tokens.iter().map(|token| (*token).to_string()))
    }

    fn store_with(task_ids: &[&str], calendars: Vec<CalendarInfo>) -> Store {
        let mut store = Store {
            data_dir: PathBuf::new(),
            resources: BTreeMap::new(),
            calendars,
            tokens: BTreeMap::new(),
            settings: AppSettings::default(),
            ui_state: serde_json::Map::new(),
            offline: false,
            last_synced: None,
        };
        for id in task_ids {
            store.resources.insert(
                format!("{id}.ics"),
                Resource {
                    href: format!("{id}.ics"),
                    collection: "/tasks/".to_string(),
                    etag: None,
                    raw: String::new(),
                    items: ResourceItems::Tasks(vec![empty_task((*id).to_string(), "task".into())]),
                },
            );
        }
        store
    }

    #[test]
    fn positionals_and_options_are_separated() {
        let parsed = cli(&["task-add", "--title", "Ship the CLI", "--due", "2026-10-28"]);

        assert_eq!(parsed.positional(0), Some("task-add"));
        assert_eq!(parsed.option("title"), Some("Ship the CLI"));
        assert_eq!(parsed.option("due"), Some("2026-10-28"));
    }

    #[test]
    fn a_boolean_flag_does_not_swallow_the_next_flag() {
        let parsed = cli(&["task-set", "a2a8", "--clear-due", "--priority", "high"]);

        assert!(parsed.has("clear-due"));
        assert_eq!(parsed.option("priority"), Some("high"));
        assert_eq!(parsed.positional(1), Some("a2a8"));
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

    #[test]
    fn an_id_prefix_resolves_to_its_task() {
        let store = store_with(&["a2a8bd04-1111", "a2a8ffff-2222"], Vec::new());

        let found = find_task(&store, "a2a8bd").unwrap();

        assert_eq!(found.id.0, "a2a8bd04-1111");
    }

    #[test]
    fn an_ambiguous_prefix_is_rejected_instead_of_picking_one() {
        let store = store_with(&["a2a8bd04-1111", "a2a8ffff-2222"], Vec::new());

        let error = find_task(&store, "a2a8").unwrap_err();

        assert!(matches!(error, AppError::InvalidInput(_)));
    }

    #[test]
    fn an_unknown_id_prefix_is_not_found() {
        let store = store_with(&["a2a8bd04-1111"], Vec::new());

        assert!(matches!(
            find_task(&store, "zz"),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn a_list_name_is_matched_ignoring_case_and_reports_its_component() {
        let store = store_with(
            &[],
            vec![
                CalendarInfo {
                    href: "/me/cal/".to_string(),
                    display_name: "SqrtCal".to_string(),
                    component: CalendarComponent::VEvent,
                    ..CalendarInfo::default()
                },
                CalendarInfo {
                    href: "/me/tasks/".to_string(),
                    display_name: "SqrtTasks".to_string(),
                    component: CalendarComponent::VTodo,
                    ..CalendarInfo::default()
                },
            ],
        );

        assert_eq!(
            resolve_list(&store, "sqrtcal").unwrap(),
            ("/me/cal/".to_string(), CalendarComponent::VEvent)
        );
        assert!(require_component(
            CalendarComponent::VEvent,
            CalendarComponent::VTodo,
            "sqrtcal"
        )
        .is_err());
        assert!(require_component(
            CalendarComponent::VTodo,
            CalendarComponent::VTodo,
            "sqrttasks"
        )
        .is_ok());
    }

    #[test]
    fn several_ids_are_read_from_one_invocation() {
        let parsed = cli(&["task-set", "a2a8", "b7c1", "c9d0", "--priority", "high"]);

        assert_eq!(
            require_ids(&parsed, "<id>").unwrap(),
            vec!["a2a8", "b7c1", "c9d0"]
        );
        assert!(require_ids(&cli(&["task-set", "--priority", "high"]), "<id>").is_err());
    }

    #[test]
    fn every_dispatched_verb_is_documented() {
        let source = include_str!("taskroot.rs");
        let arms = source
            .split_once("match command {")
            .and_then(|(_, rest)| rest.split_once("other =>"))
            .map(|(arms, _)| arms)
            .unwrap();

        let mut dispatched: Vec<&str> = arms
            .lines()
            .filter_map(|line| line.trim().strip_prefix('"'))
            .filter_map(|rest| rest.split_once("\" =>").map(|(name, _)| name))
            .collect();
        let mut documented: Vec<&str> = VERBS
            .iter()
            .filter_map(|verb| verb.usage.split_whitespace().next())
            .collect();
        dispatched.sort_unstable();
        documented.sort_unstable();

        assert_eq!(dispatched, documented);
    }

    #[test]
    fn exports_the_cli_reference() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/cli.md");

        std::fs::write(path, help_markdown()).unwrap();
    }
}
