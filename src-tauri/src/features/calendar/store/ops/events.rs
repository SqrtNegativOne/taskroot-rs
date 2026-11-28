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
    apply_resource_create, apply_resource_move, apply_resource_update, collection_for,
    ensure_source_writable, refetch_resource, require_uid, resource_href, writable_calendar,
    UpdateGuard,
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
        let new_etag = client.put_resource(&new_href, &merged, None, true).await?;
        client
            .delete_resource(&resource.href, resource.etag.as_deref())
            .await?;
        return state
            .write(|store| {
                apply_resource_move(
                    store,
                    &resource.href,
                    &new_href,
                    &target.href,
                    base_revision,
                    resource.etag.as_deref(),
                    new_etag.as_deref(),
                    &merged,
                )
            })
            .await;
    }

    let (final_raw, final_etag, committed_from) =
        put_event_with_retry(client, &resource, id, patches, merged).await?;

    state
        .write(|store| {
            apply_resource_update(
                store,
                &resource.href,
                &resource.collection,
                UpdateGuard {
                    base_revision,
                    initial_etag: resource.etag.as_deref(),
                    committed_from_etag: committed_from.as_deref(),
                },
                final_etag.as_deref(),
                &final_raw,
            )
        })
        .await
}

async fn put_event_with_retry(
    client: &CalDavClient,
    resource: &Resource,
    id: &str,
    patches: &[EventPatch],
    merged: String,
) -> Result<(String, Option<String>, Option<String>), AppError> {
    match client
        .put_resource(&resource.href, &merged, resource.etag.as_deref(), false)
        .await
    {
        Ok(etag) => Ok((
            merged,
            etag.or_else(|| resource.etag.clone()),
            resource.etag.clone(),
        )),
        Err(AppError::Conflict(_)) => {
            let fresh = refetch_resource(client, &resource.collection, &resource.href)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("event {id}")))?;
            let mut fresh_event = find_event_in(&fresh, id)?.clone();
            apply_event_patches(&mut fresh_event, patches);
            let retried = ics::write_event_resource(&fresh.raw, &fresh_event)?;
            let etag = client
                .put_resource(&resource.href, &retried, fresh.etag.as_deref(), false)
                .await?;
            Ok((
                retried,
                etag.or_else(|| fresh.etag.clone()),
                fresh.etag.clone(),
            ))
        }
        Err(error) => Err(error),
    }
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
/// occurrence, so the rest of the series keeps its slot; [`EventScope::All`]
/// moves the master. Moving a series from an occurrence is not supported.
///
/// # Errors
///
/// Returns [`AppError::NotFound`] for an unknown event,
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
    let event = state
        .read(|store| store.event(&id).cloned())
        .await
        .ok_or_else(|| AppError::NotFound(format!("event {id}")))?;
    if !event.is_recurring() || scope == EventScope::All || event.recurring_event_id.is_some() {
        let patches = [
            EventPatch::StartTime(start_time),
            EventPatch::EndTime(end_time),
            EventPatch::IsAllDay(Some(is_all_day)),
        ];
        return update_event(state, client, &id, &patches).await;
    }
    if scope == EventScope::Following {
        return Err(AppError::InvalidInput(
            "moving a series from one occurrence is not supported".to_string(),
        ));
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

    let (final_raw, final_etag, committed_from) = match client
        .put_resource(&resource.href, &merged, resource.etag.as_deref(), false)
        .await
    {
        Ok(etag) => (
            merged,
            etag.or_else(|| resource.etag.clone()),
            resource.etag.clone(),
        ),
        Err(AppError::Conflict(_)) => {
            let fresh = refetch_resource(client, &resource.collection, &resource.href)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("event {master_id}")))?;
            let fresh_master = find_event_in(&fresh, master_id)?;
            let exception =
                exception_for(fresh_master, occurrence, start_time, end_time, is_all_day);
            let retried = ics::write_event_resource(&fresh.raw, &exception)?;
            let etag = client
                .put_resource(&resource.href, &retried, fresh.etag.as_deref(), false)
                .await?;
            (
                retried,
                etag.or_else(|| fresh.etag.clone()),
                fresh.etag.clone(),
            )
        }
        Err(error) => return Err(error),
    };

    state
        .write(|store| {
            apply_resource_update(
                store,
                &resource.href,
                &resource.collection,
                UpdateGuard {
                    base_revision,
                    initial_etag: resource.etag.as_deref(),
                    committed_from_etag: committed_from.as_deref(),
                },
                final_etag.as_deref(),
                &final_raw,
            )
        })
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
