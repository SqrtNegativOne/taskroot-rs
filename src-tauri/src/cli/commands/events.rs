//! Event verbs: list, add, remove.

use crate::caldav::model::CalendarComponent;
use crate::domain::{AppEvent, CollectionId, EventId, EventScope, EventStatus};
use crate::error::AppError;
use crate::store::{ops, query};

use super::super::args::Cli;
use super::super::output::{plus_days, print_json, today};
use super::super::session::open;
use super::{require_component, resolve_list};

/// # Errors
///
/// Propagates session, query, projection and output failures.
pub async fn list_events(cli: &Cli) -> Result<(), AppError> {
    let session = open().await?;
    let search = cli.option("search").unwrap_or_default();
    let events = query::query_events(&session.store.events(), &[], search);
    let start = cli.option("start").map_or_else(today, ToString::to_string);
    let end = cli
        .option("end")
        .map_or_else(|| plus_days(14), ToString::to_string);
    print_json(&crate::domain::projection::expand_event_instances(
        &events, &start, &end,
    ))
}

/// # Errors
///
/// Propagates session, option, list-resolution and write failures.
pub async fn add_event(cli: &Cli) -> Result<(), AppError> {
    let mut session = open().await?;
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
        is_all_day: Some(cli.has("all-day")),
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

/// # Errors
///
/// Propagates session, delete and output failures.
pub async fn remove_event(cli: &Cli) -> Result<(), AppError> {
    let mut session = open().await?;
    let id = cli.require_positional(1, "<id>")?;
    ops::delete_event(
        &mut session.store,
        &session.client,
        id,
        EventScope::All,
        None,
    )
    .await?;
    print_json(&serde_json::json!({ "deleted": id }))
}
