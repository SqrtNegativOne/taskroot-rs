//! Live Radicale end-to-end check (P4/P6 verification).
//!
//! Creates, edits/moves and deletes one event and one task through the same
//! `Store`/`ops` code the app commands use, reloads the store from the server,
//! and compares the reloaded bodies with the server. It also edits and then
//! restores a real tasks.org VTODO to prove `X-APPLE-SORT-ORDER` survives.
//!
//! Run with `TASKROOT_DATA_DIR=target/e2e-data`:
//! `cargo run --example e2e_caldav -- --user me --password ...`

use std::error::Error;
use std::path::{Path, PathBuf};

use taskroot_lib::features::calendar::CalDavClient;
use taskroot_lib::features::calendar::discovery;
use taskroot_lib::features::calendar::HttpClient;
use taskroot_lib::features::calendar::{
    AppEvent, AppTask, AppTaskStatus, Color, EventId, EventPatch, EventScope, EventStatus, TaskId,
    TaskPatch, TaskPriority,
};
use taskroot_lib::features::calendar::{ops, Store, StoreState};

struct Args {
    server: String,
    user: String,
    password: String,
}

fn e2e_task(task_id: &str) -> AppTask {
    AppTask {
        id: TaskId::from(task_id),
        title: "Taskroot E2E task".to_string(),
        status: Some(AppTaskStatus::Todo),
        priority: Some(TaskPriority::High),
        tags: None,
        parent_task: None,
        dependencies: None,
        est: Some(30),
        time_spent: None,
        added: None,
        remote_id: None,
        remote_collection_id: None,
        notes: Some("created by the migration E2E check".to_string()),
        start: None,
        due: Some("2026-12-02".parse().expect("valid date")),
        completed_at: None,
        rrule: None,
        geo: None,
        alarms: None,
        timezone: None,
        updated_at: None,
        etag: None,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = parse_args()?;
    let http = HttpClient::new()?;
    let discovered = discovery::discover(&http, &args.server, &args.user, &args.password).await?;
    let client = CalDavClient::new(&http, &discovered.server_url, &args.user, &args.password)?;

    let dir = data_dir();
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;

    let mut store = Store::load(dir.clone());
    store.calendars = discovered.calendars.clone();
    ops::full_load(&mut store, &client).await?;
    println!("loaded {} resources from Radicale", store.resources.len());
    let state = StoreState::new(store);

    exercise_event(&state, &client).await?;
    exercise_task(&state, &client).await?;
    preserve_apple_sort_order(&state, &client).await?;
    restart_and_compare(&client, &dir, &discovered.calendars).await?;
    println!("E2E OK");
    Ok(())
}

async fn exercise_event(state: &StoreState, client: &CalDavClient) -> Result<(), Box<dyn Error>> {
    let event_id = format!("e2e-{}", uuid::Uuid::new_v4());
    let event = AppEvent {
        id: EventId::from(event_id.as_str()),
        remote_id: None,
        remote_collection_id: None,
        task_id: None,
        title: "Taskroot E2E event".to_string(),
        description: Some("created by the migration E2E check".to_string()),
        priority: None,
        tags: None,
        start_time: "2026-12-01T10:00:00".parse()?,
        end_time: "2026-12-01T11:00:00".parse()?,
        rrule: None,
        exdates: None,
        recurring_event_id: None,
        original_start_time: None,
        status: Some(EventStatus::Confirmed),
        updated_at: None,
        added: None,
        color: Color::try_from("#4285f4".to_string()).ok(),
        etag: None,
        is_all_day: Some(false),
        timezone: Some("Asia/Kolkata".to_string()),
    };

    let event = ops::create_event(state, client, event).await?;
    let href = event
        .remote_id
        .as_deref()
        .ok_or("created event has no href")?
        .clone();
    assert_exists(client, &href).await?;
    println!("created event {href}");

    ops::update_event(
        state,
        client,
        event.id.as_str(),
        &[
            EventPatch::Title("Taskroot E2E event edited".to_string()),
            EventPatch::StartTime("2026-12-01T15:30:00".parse()?),
            EventPatch::EndTime("2026-12-01T16:30:00".parse()?),
        ],
    )
    .await?;
    let raw = fetch_one(client, &href).await?;
    ensure(
        raw.contains("SUMMARY:Taskroot E2E event edited"),
        "the edited summary did not reach the server",
    )?;
    ensure(
        raw.contains("DTSTART;TZID=Asia/Kolkata:20261201T153000"),
        "the moved start did not reach the server",
    )?;
    println!("edited and moved event");

    ops::delete_event(state, client, event_id.as_str(), EventScope::All, None).await?;
    assert_absent(client, &href).await?;
    println!("deleted event");
    Ok(())
}

async fn exercise_task(state: &StoreState, client: &CalDavClient) -> Result<(), Box<dyn Error>> {
    let task_id = format!("e2e-{}", uuid::Uuid::new_v4());
    let task = ops::create_task(state, client, e2e_task(&task_id)).await?;
    let task_href = task
        .remote_id
        .as_deref()
        .ok_or("created task has no href")?
        .clone();
    assert_exists(client, &task_href).await?;
    println!("created task {task_href}");

    ops::update_task(
        state,
        client,
        task.id.as_str(),
        &[
            TaskPatch::Title("Taskroot E2E task edited".to_string()),
            TaskPatch::Status(Some(AppTaskStatus::Done)),
            TaskPatch::Due(Some("2026-12-05".parse()?)),
        ],
    )
    .await?;
    let raw = fetch_one(client, &task_href).await?;
    ensure(
        raw.contains("SUMMARY:Taskroot E2E task edited"),
        "the edited task summary did not reach the server",
    )?;
    ensure(
        raw.contains("STATUS:COMPLETED"),
        "the completed status did not reach the server",
    )?;
    ensure(
        raw.contains("DUE;VALUE=DATE:20261205"),
        "the rescheduled due date did not reach the server",
    )?;
    println!("edited and rescheduled task");

    ops::delete_task(state, client, task_id.as_str()).await?;
    assert_absent(client, &task_href).await?;
    println!("deleted task");
    Ok(())
}

async fn preserve_apple_sort_order(
    state: &StoreState,
    client: &CalDavClient,
) -> Result<(), Box<dyn Error>> {
    let task = state
        .read(Store::tasks)
        .await
        .into_iter()
        .find(|task| task.title.eq_ignore_ascii_case("brush"))
        .ok_or("the tasks.org Brush fixture is missing from Radicale")?;
    let original_title = task.title.clone();
    let href = task
        .remote_id
        .as_deref()
        .ok_or("the tasks.org task has no href")?
        .clone();
    let before = fetch_one(client, &href).await?;
    ensure(
        before.contains("X-APPLE-SORT-ORDER:"),
        "fixture has no sort order",
    )?;

    ops::update_task(
        state,
        client,
        task.id.as_str(),
        &[TaskPatch::Title(format!("{original_title} (e2e)"))],
    )
    .await?;
    let after = fetch_one(client, &href).await?;
    ensure(
        after.contains("X-APPLE-SORT-ORDER:"),
        format!("the app edit dropped X-APPLE-SORT-ORDER:\n{after}"),
    )?;
    ensure(
        after.contains("SUMMARY:Brush (e2e)"),
        "the renamed summary did not reach the server",
    )?;

    ops::update_task(
        state,
        client,
        task.id.as_str(),
        &[TaskPatch::Title(original_title)],
    )
    .await?;
    println!("X-APPLE-SORT-ORDER survives an app edit");
    Ok(())
}

async fn restart_and_compare(
    client: &CalDavClient,
    dir: &Path,
    calendars: &[taskroot_lib::features::calendar::CalendarInfo],
) -> Result<(), Box<dyn Error>> {
    let mut restarted = Store::load(dir.to_path_buf());
    restarted.calendars = calendars.to_vec();
    ops::full_load(&mut restarted, client).await?;

    let hrefs: Vec<String> = restarted.resources.keys().cloned().collect();
    for href in &hrefs {
        let live = fetch_one(client, href).await?;
        let loaded = restarted
            .resources
            .get(href)
            .map(|resource| resource.raw.as_str())
            .ok_or_else(|| format!("{href} disappeared from the reloaded store"))?;
        ensure(
            live == loaded,
            format!("reloaded resource {href} does not match the server"),
        )?;
    }
    println!(
        "restart: reloaded {} resources and the server agrees",
        restarted.resources.len()
    );
    Ok(())
}

async fn fetch_one(client: &CalDavClient, href: &str) -> Result<String, Box<dyn Error>> {
    let collection = href
        .rsplit_once('/')
        .map(|(collection, _)| format!("{collection}/"))
        .ok_or("href has no collection")?;
    let resources = client.fetch_hrefs(&collection, &[href.to_string()]).await?;
    resources
        .into_iter()
        .find_map(|resource| resource.calendar_data)
        .ok_or_else(|| format!("{href} was not returned by the server").into())
}

fn ensure(condition: bool, message: impl Into<String>) -> Result<(), Box<dyn Error>> {
    if condition {
        return Ok(());
    }
    Err(message.into().into())
}

async fn assert_exists(client: &CalDavClient, href: &str) -> Result<(), Box<dyn Error>> {
    fetch_one(client, href).await?;
    Ok(())
}

async fn assert_absent(client: &CalDavClient, href: &str) -> Result<(), Box<dyn Error>> {
    match fetch_one(client, href).await {
        Ok(_) => Err(format!("{href} still exists").into()),
        Err(_) => Ok(()),
    }
}

fn data_dir() -> PathBuf {
    std::env::var_os("TASKROOT_DATA_DIR")
        .map_or_else(|| PathBuf::from("target/e2e-data"), PathBuf::from)
}

fn parse_args() -> Result<Args, Box<dyn Error>> {
    let mut server = discovery::DEFAULT_SERVER_URL.to_string();
    let mut user = "me".to_string();
    let mut password = std::env::var("RADICALE_PASSWORD").unwrap_or_default();

    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--server" => server = next(&mut args, "--server")?,
            "--user" => user = next(&mut args, "--user")?,
            "--password" => password = next(&mut args, "--password")?,
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }
    if password.is_empty() {
        return Err("pass --password or set RADICALE_PASSWORD".into());
    }
    Ok(Args {
        server,
        user,
        password,
    })
}

fn next(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, Box<dyn Error>> {
    args.next()
        .ok_or_else(|| format!("missing value for {flag}").into())
}
