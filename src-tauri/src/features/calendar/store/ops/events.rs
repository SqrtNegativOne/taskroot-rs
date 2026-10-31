//! Event writes: create/update/delete plus scoped recurrence resolution.

use crate::features::calendar::caldav::client::CalDavClient;
use crate::features::calendar::caldav::model::CalendarComponent;
use crate::features::calendar::domain::{AppEvent, CollectionId, EventId, EventScope, RemoteId};
use crate::error::AppError;
use crate::features::calendar::ics;

use super::super::model::{Resource, Store};
use super::{collection_for, refetch_resource, require_uid, resource_href, writable_calendar};

/// Create one event resource.
///
/// # Errors
///
/// Returns an error for an empty id, a read-only/unknown target calendar, or a
/// failed `PUT`.
pub async fn create_event(
    store: &mut Store,
    client: &CalDavClient,
    mut event: AppEvent,
) -> Result<AppEvent, AppError> {
    let collection = collection_for(
        store,
        CalendarComponent::VEvent,
        event.remote_collection_id.as_ref().map(|id| id.as_str()),
    )?;
    let uid = event.id.as_str().to_string();
    require_uid(&uid)?;
    let href = resource_href(&collection.href, &uid);
    let raw = ics::create_event_resource(&event)?;
    let etag = client.put_resource(&href, &raw, None, true).await?;

    event.remote_id = Some(RemoteId(href.clone()));
    event.remote_collection_id = Some(CollectionId(collection.href.clone()));
    store.set_resource(&href, &collection.href, etag.as_deref(), &raw)?;
    Ok(event)
}

/// Update one event resource, moving it when its calendar changed.
///
/// # Errors
///
/// Returns [`AppError::NotFound`] when the event is not in the store,
/// [`AppError::InvalidInput`] for a read-only target, and propagates transport
/// errors (a second `412` is returned, never swallowed).
pub async fn update_event(
    store: &mut Store,
    client: &CalDavClient,
    event: AppEvent,
) -> Result<(), AppError> {
    let resource = resource_for_event(store, &event)?;
    let target = match event.remote_collection_id.as_deref() {
        Some(href) => writable_calendar(store, href, CalendarComponent::VEvent)?,
        None => collection_for(store, CalendarComponent::VEvent, None)?,
    };

    let merged = ics::write_event_resource(&resource.raw, &event)?;

    if target.href != resource.collection {
        let href = resource_href(&target.href, event.id.as_str());
        let etag = client.put_resource(&href, &merged, None, true).await?;
        client
            .delete_resource(&resource.href, resource.etag.as_deref())
            .await?;
        store.remove_resource(&resource.href);
        store.set_resource(&href, &target.href, etag.as_deref(), &merged)?;
        return Ok(());
    }

    let (merged, etag) = match client
        .put_resource(&resource.href, &merged, resource.etag.as_deref(), false)
        .await
    {
        Ok(etag) => (merged, etag.or_else(|| resource.etag.clone())),
        Err(AppError::Conflict(_)) => {
            let fresh = refetch_resource(client, store, &resource)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("event {}", event.id)))?;
            let merged = ics::write_event_resource(&fresh.raw, &event)?;
            let etag = client
                .put_resource(&resource.href, &merged, fresh.etag.as_deref(), false)
                .await?;
            (merged, etag.or_else(|| fresh.etag.clone()))
        }
        Err(error) => return Err(error),
    };

    store.set_resource(
        &resource.href,
        &resource.collection,
        etag.as_deref(),
        &merged,
    )?;
    Ok(())
}

/// Delete a recurring event's occurrences according to `scope`.
///
/// [`EventScope::All`] removes the whole series, [`EventScope::This`] excludes
/// only `occurrence_start` (and drops the exception row when that is what was
/// selected), and [`EventScope::Following`] ends the rule just before it.
///
/// # Errors
///
/// Propagates transport errors from the delete or the rewrite.
pub async fn delete_event(
    store: &mut Store,
    client: &CalDavClient,
    id: &str,
    scope: EventScope,
    occurrence_start: Option<&str>,
) -> Result<(), AppError> {
    let Some((resource, event)) = series_resource(store, id) else {
        return Ok(());
    };
    let master = master_event(store, &event);
    let is_series = master.as_ref().is_some_and(AppEvent::is_recurring);

    if scope == EventScope::All || !is_series {
        return match master {
            Some(master) => delete_series(store, client, &resource, &master).await,
            None => delete_resource(store, client, &resource).await,
        };
    }

    let occurrence = selected_occurrence(&event, occurrence_start);
    let Some(master) = master else {
        return Err(AppError::NotFound(format!("series of event {id}")));
    };
    let mut raw = resource.raw.clone();

    if scope == EventScope::This && event.recurring_event_id.is_some() {
        let Some(remaining) = ics::delete_event_component(&raw, &event)? else {
            return delete_resource(store, client, &resource).await;
        };
        raw = remaining;
    }

    if scope == EventScope::Following {
        for victim in exceptions_from(&resource, &occurrence) {
            if let Some(remaining) = ics::delete_event_component(&raw, &victim)? {
                raw = remaining;
            }
        }
    }

    let updated = scoped_master(&master, scope, &occurrence)?;
    let raw = ics::write_event_resource(&raw, &updated)?;
    put_resource(store, client, &resource, &raw).await
}

/// A requested move of one occurrence or of a whole series.
#[derive(Debug, Clone)]
pub struct Reschedule {
    pub id: String,
    pub start_time: String,
    pub end_time: String,
    pub is_all_day: bool,
    pub scope: EventScope,
    pub occurrence_start: Option<String>,
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
    store: &mut Store,
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
    let Some(event) = store.event(&id).cloned() else {
        return Err(AppError::NotFound(format!("event {id}")));
    };
    if !event.is_recurring() || scope == EventScope::All {
        let moved = moved(event, start_time, end_time, is_all_day);
        return update_event(store, client, moved).await;
    }
    if scope == EventScope::Following {
        return Err(AppError::InvalidInput(
            "moving a series from one occurrence is not supported".to_string(),
        ));
    }
    if event.recurring_event_id.is_some() {
        let moved = moved(event, start_time, end_time, is_all_day);
        return update_event(store, client, moved).await;
    }
    let occurrence = selected_occurrence(&event, occurrence_start.as_deref());
    let exception = exception_for(&event, &occurrence, start_time, end_time, is_all_day);
    update_event(store, client, exception).await
}

fn series_resource(store: &Store, id: &str) -> Option<(Resource, AppEvent)> {
    let resource = store.resource_for_event(id)?.clone();
    let event = resource
        .as_events()?
        .iter()
        .find(|event| event.id.as_str() == id)?
        .clone();
    Some((resource, event))
}

fn master_event(store: &Store, event: &AppEvent) -> Option<AppEvent> {
    event
        .recurring_event_id
        .as_deref()
        .map_or_else(|| Some(event.clone()), |master_id| store.event(master_id).cloned())
}

/// The canonical occurrence a mutation targets: the caller's value when it has
/// one, otherwise the row's own start (a master starts at its first occurrence).
fn selected_occurrence(event: &AppEvent, occurrence_start: Option<&str>) -> String {
    occurrence_start
        .or(event.original_start_time.as_deref())
        .unwrap_or(&event.start_time)
        .to_string()
}

/// Exceptions whose original slot is at or after `occurrence`.
fn exceptions_from(resource: &Resource, occurrence: &str) -> Vec<AppEvent> {
    resource
        .as_events()
        .unwrap_or_default()
        .iter()
        .filter(|event| event.recurring_event_id.is_some())
        .filter(|event| event.original_start_time.as_deref().is_some_and(|start| start >= occurrence))
        .cloned()
        .collect()
}

/// The master row as it must be written after a single-occurrence or
/// from-here-on edit: an added `EXDATE`, or a rule that ends before `occurrence`.
fn scoped_master(
    master: &AppEvent,
    scope: EventScope,
    occurrence: &str,
) -> Result<AppEvent, AppError> {
    let mut updated = master.clone();
    if scope == EventScope::Following {
        if let Some(rrule) = master.rrule.as_deref() {
            updated.rrule = Some(crate::features::calendar::domain::timing::truncate_before(
                rrule,
                occurrence,
                master.timezone.as_deref(),
            )?);
        }
        return Ok(updated);
    }
    let mut exdates = master.exdates.clone().unwrap_or_default();
    if !exdates.iter().any(|value| value == occurrence) {
        exdates.push(occurrence.to_string());
    }
    updated.exdates = Some(exdates);
    Ok(updated)
}

async fn delete_series(
    store: &mut Store,
    client: &CalDavClient,
    resource: &Resource,
    master: &AppEvent,
) -> Result<(), AppError> {
    match ics::delete_event_component(&resource.raw, master)? {
        Some(raw) => put_resource(store, client, resource, &raw).await,
        None => delete_resource(store, client, resource).await,
    }
}

async fn delete_resource(
    store: &mut Store,
    client: &CalDavClient,
    resource: &Resource,
) -> Result<(), AppError> {
    client
        .delete_resource(&resource.href, resource.etag.as_deref())
        .await?;
    store.remove_resource(&resource.href);
    Ok(())
}

async fn put_resource(
    store: &mut Store,
    client: &CalDavClient,
    resource: &Resource,
    raw: &str,
) -> Result<(), AppError> {
    let etag = client
        .put_resource(&resource.href, raw, resource.etag.as_deref(), false)
        .await?;
    store.set_resource(
        &resource.href,
        &resource.collection,
        etag.as_deref().or(resource.etag.as_deref()),
        raw,
    )
}

fn moved(mut event: AppEvent, start_time: String, end_time: String, is_all_day: bool) -> AppEvent {
    event.start_time = start_time;
    event.end_time = end_time;
    event.is_all_day = Some(is_all_day);
    event
}

/// A `VCALENDAR` exception row for one occurrence of `master`.
fn exception_for(
    master: &AppEvent,
    occurrence: &str,
    start_time: String,
    end_time: String,
    is_all_day: bool,
) -> AppEvent {
    AppEvent {
        id: EventId::from(format!("{}@{occurrence}", master.id)),
        remote_id: master.remote_id.clone(),
        remote_collection_id: master.remote_collection_id.clone(),
        task_id: master.task_id.clone(),
        title: master.title.clone(),
        description: master.description.clone(),
        start_time,
        end_time,
        rrule: None,
        exdates: None,
        recurring_event_id: Some(master.id.clone()),
        original_start_time: Some(occurrence.to_string()),
        status: master.status.clone(),
        updated_at: None,
        color: master.color.clone(),
        etag: None,
        is_all_day: Some(is_all_day),
        timezone: master.timezone.clone(),
    }
}

fn resource_for_event(store: &Store, event: &AppEvent) -> Result<Resource, AppError> {
    event
        .remote_id
        .as_deref()
        .and_then(|href| store.resources.get(href))
        .or_else(|| store.resource_for_event(event.id.as_str()))
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("event {}", event.id)))
}
