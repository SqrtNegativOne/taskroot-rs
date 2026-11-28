//! Event writes: create, patch-based update, and scoped reschedule.

use crate::error::AppError;
use crate::features::calendar::caldav::client::CalDavClient;
use crate::features::calendar::caldav::model::CalendarComponent;
use crate::features::calendar::domain::{
    apply_event_patches, AppEvent, CalendarDateTime, CollectionId, EventId, EventPatch, EventScope,
    RemoteId,
};
use crate::features::calendar::ics;

use super::super::model::Resource;
use super::super::state::StoreState;
use super::{
    apply_resource_create, collection_for, ensure_source_writable, move_resource,
    put_with_conflict_retry, require_uid, resolve_master, resource_href, writable_calendar,
};

/// Create one event resource without holding the store lock during network I/O.
///
/// # Errors
///
/// Returns an error for an empty id, a read-only/unknown target calendar, or a
/// failed `PUT`.
pub async fn create_event(
    state: &StoreState,
    client: &CalDavClient,
    mut event: AppEvent,
) -> Result<AppEvent, AppError> {
    let (collection, href, raw, base_revision) = state
        .read(|store| {
            let collection = collection_for(
                store,
                CalendarComponent::VEvent,
                event.remote_collection_id.as_ref().map(|id| id.as_str()),
            )?;
            let uid = event.id.as_str();
            require_uid(uid)?;
            let href = resource_href(&collection.href, uid);
            let raw = ics::create_event_resource(&event)?;
            Ok::<_, AppError>((collection, href, raw, store.revision()))
        })
        .await?;

    let etag = client.put_resource(&href, &raw, None, true).await?;

    event.remote_id = Some(RemoteId(href.clone()));
    event.remote_collection_id = Some(CollectionId(collection.href.clone()));
    state
        .write(|store| {
            apply_resource_create(
                store,
                &href,
                &collection.href,
                base_revision,
                etag.as_deref(),
                &raw,
            )
        })
        .await?;
    Ok(event)
}

/// Apply field `patches` to one event, moving it when its calendar changed.
///
/// On `412 Conflict`, refetches the fresh server resource, applies only
/// `patches` onto the freshly parsed event, and retries the `PUT` once (a second
/// `412` is returned, never swallowed).
///
/// # Errors
///
/// Returns [`AppError::NotFound`] when the event is not in the store,
/// [`AppError::InvalidInput`] for a read-only target, and propagates transport
/// errors.
pub async fn update_event(
    state: &StoreState,
    client: &CalDavClient,
    id: &str,
    patches: &[EventPatch],
) -> Result<(), AppError> {
    let (resource, target, merged, base_revision) = state
        .read(|store| {
            let resource = store
                .resource_for_event(id)
                .cloned()
                .ok_or_else(|| AppError::NotFound(format!("event {id}")))?;
            let mut event = find_event_in(&resource, id)?.clone();
            apply_event_patches(&mut event, patches);
            let target = match event.remote_collection_id.as_deref() {
                Some(href) => writable_calendar(store, href, CalendarComponent::VEvent)?,
                None => collection_for(store, CalendarComponent::VEvent, None)?,
            };
            if target.href != resource.collection {
                ensure_source_writable(store, &resource.collection)?;
            }
            let merged = ics::write_event_resource(&resource.raw, &event)?;
            Ok::<_, AppError>((resource, target, merged, store.revision()))
        })
        .await?;

    if target.href != resource.collection {
        let new_href = resource_href(&target.href, id);
        return move_resource(
            state,
            client,
            &resource,
            &target.href,
            &new_href,
            &merged,
            base_revision,
        )
        .await;
    }

    put_with_conflict_retry(
        state,
        client,
        &resource,
        base_revision,
        merged,
        &format!("event {id}"),
        |fresh| {
            let mut fresh_event = find_event_in(fresh, id)?.clone();
            apply_event_patches(&mut fresh_event, patches);
            ics::write_event_resource(&fresh.raw, &fresh_event)
        },
    )
    .await
}

/// A requested move of one occurrence or of a whole series.
#[derive(Debug, Clone)]
pub struct Reschedule {
    pub id: String,
    pub start_time: CalendarDateTime,
    pub end_time: CalendarDateTime,
    pub is_all_day: bool,
    pub scope: EventScope,
    pub occurrence_start: Option<CalendarDateTime>,
}

/// Move one occurrence, or the whole series, to a new time.
///
/// [`EventScope::This`] writes an exception (`RECURRENCE-ID`) for the selected
/// occurrence (or updates an existing exception row directly); [`EventScope::All`]
/// resolves and moves the series master even when invoked from an exception row.
/// Moving a series from one occurrence (`following`) is not supported.
///
/// # Errors
///
/// Returns [`AppError::NotFound`] for an unknown event or missing master,
/// [`AppError::InvalidInput`] for a `following` move, and propagates transport
/// errors.
pub async fn reschedule_event(
    state: &StoreState,
    client: &CalDavClient,
    request: Reschedule,
) -> Result<(), AppError> {
    let Reschedule {
        id,
        start_time,
        end_time,
        is_all_day,
        scope,
        occurrence_start,
    } = request;
    let (event, master) = state
        .read(|store| {
            let event = store
                .event(&id)
                .cloned()
                .ok_or_else(|| AppError::NotFound(format!("event {id}")))?;
            let master = resolve_master(store, &event);
            Ok::<_, AppError>((event, master))
        })
        .await?;

    let is_series = master.as_ref().is_some_and(AppEvent::is_recurring);
    if scope == EventScope::Following && is_series {
        return Err(AppError::InvalidInput(
            "moving a series from one occurrence is not supported".to_string(),
        ));
    }

    if scope == EventScope::All {
        let target_id = if event.recurring_event_id.is_some() {
            master
                .ok_or_else(|| AppError::NotFound(format!("series of event {id}")))?
                .id
                .to_string()
        } else {
            id
        };
        let patches = [
            EventPatch::StartTime(start_time),
            EventPatch::EndTime(end_time),
            EventPatch::IsAllDay(Some(is_all_day)),
        ];
        return update_event(state, client, &target_id, &patches).await;
    }

    if !event.is_recurring() || event.recurring_event_id.is_some() {
        let patches = [
            EventPatch::StartTime(start_time),
            EventPatch::EndTime(end_time),
            EventPatch::IsAllDay(Some(is_all_day)),
        ];
        return update_event(state, client, &id, &patches).await;
    }

    let occurrence = selected_occurrence(&event, occurrence_start);
    write_exception(
        state,
        client,
        &id,
        occurrence,
        start_time,
        end_time,
        is_all_day,
    )
    .await
}

async fn write_exception(
    state: &StoreState,
    client: &CalDavClient,
    master_id: &str,
    occurrence: CalendarDateTime,
    start_time: CalendarDateTime,
    end_time: CalendarDateTime,
    is_all_day: bool,
) -> Result<(), AppError> {
    let (resource, merged, base_revision) = state
        .read(|store| {
            let resource = store
                .resource_for_event(master_id)
                .cloned()
                .ok_or_else(|| AppError::NotFound(format!("event {master_id}")))?;
            writable_calendar(store, &resource.collection, CalendarComponent::VEvent)?;
            let master = find_event_in(&resource, master_id)?;
            let exception = exception_for(master, occurrence, start_time, end_time, is_all_day);
            let merged = ics::write_event_resource(&resource.raw, &exception)?;
            Ok::<_, AppError>((resource, merged, store.revision()))
        })
        .await?;

    put_with_conflict_retry(
        state,
        client,
        &resource,
        base_revision,
        merged,
        &format!("event {master_id}"),
        |fresh| {
            let fresh_master = find_event_in(fresh, master_id)?;
            let exception =
                exception_for(fresh_master, occurrence, start_time, end_time, is_all_day);
            ics::write_event_resource(&fresh.raw, &exception)
        },
    )
    .await
}

pub(super) fn selected_occurrence(
    event: &AppEvent,
    occurrence_start: Option<CalendarDateTime>,
) -> CalendarDateTime {
    occurrence_start
        .or(event.original_start_time)
        .unwrap_or(event.start_time)
}

fn find_event_in<'a>(resource: &'a Resource, id: &str) -> Result<&'a AppEvent, AppError> {
    resource
        .as_events()
        .and_then(|events| events.iter().find(|event| event.id.as_str() == id))
        .ok_or_else(|| AppError::NotFound(format!("event {id}")))
}

fn exception_for(
    master: &AppEvent,
    occurrence: CalendarDateTime,
    start_time: CalendarDateTime,
    end_time: CalendarDateTime,
    is_all_day: bool,
) -> AppEvent {
    AppEvent {
        id: EventId::from(format!("{}@{occurrence}", master.id)),
        remote_id: master.remote_id.clone(),
        remote_collection_id: master.remote_collection_id.clone(),
        task_id: master.task_id.clone(),
        title: master.title.clone(),
        description: master.description.clone(),
        priority: master.priority.clone(),
        tags: master.tags.clone(),
        start_time,
        end_time,
        rrule: None,
        exdates: None,
        recurring_event_id: Some(master.id.clone()),
        original_start_time: Some(occurrence),
        status: master.status.clone(),
        updated_at: None,
        added: None,
        color: master.color.clone(),
        etag: None,
        is_all_day: Some(is_all_day),
        timezone: master.timezone.clone(),
    }
}
