//! Event verbs: list, add, remove.

use crate::error::AppError;
use crate::features::calendar::CalendarComponent;
use crate::features::calendar::{ops, query_event_instances};
use crate::features::calendar::{
    AppEvent, CollectionId, EventId, EventScope, EventStatus, FilterGroup,
};

use super::super::output::{plus_days, print_json, today};
use super::super::session::open;
use super::super::spec::{EventAddArgs, ListEventsArgs};
use super::{require_component, resolve_list};

/// # Errors
///
/// Propagates session, query, projection and output failures.
pub async fn list_events(args: &ListEventsArgs) -> Result<(), AppError> {
    let session = open().await?;
    let search = args.search.as_deref().unwrap_or_default();
    let events = session.store.events();
    let start = args.start.as_deref().map_or_else(today, ToString::to_string);
    let end = args
        .end
        .as_deref()
        .map_or_else(|| plus_days(14), ToString::to_string);
    print_json(&query_event_instances(
        &events,
        &FilterGroup::default(),
        &[],
        search,
        Some((&start, &end)),
    ))
}

/// # Errors
///
/// Propagates session, option, list-resolution and write failures.
pub async fn add_event(args: &EventAddArgs) -> Result<(), AppError> {
    let mut session = open().await?;
    let mut event = AppEvent {
        id: EventId(uuid::Uuid::new_v4().to_string()),
        remote_id: None,
        remote_collection_id: None,
        task_id: None,
        title: args.title.clone(),
        description: args.notes.clone(),
        priority: args.priority.map(Into::into),
        tags: None,
        start_time: args.start.clone(),
        end_time: args.end.clone(),
        rrule: args.rrule.clone(),
        exdates: None,
        recurring_event_id: None,
        original_start_time: None,
        status: Some(EventStatus::Confirmed),
        updated_at: None,
        added: None,
        color: None,
        etag: None,
        is_all_day: Some(args.all_day),
        timezone: args.tz.clone(),
    };

    if let Some(list) = args.list.as_deref() {
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
pub async fn remove_event(id: &str) -> Result<(), AppError> {
    let mut session = open().await?;
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
