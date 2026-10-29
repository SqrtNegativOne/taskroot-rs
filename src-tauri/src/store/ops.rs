//! Store operations: load/pull from `CalDAV`, and create/update/delete resources.
//!
//! These take the lock-free `Store` and a client so the command layer can copy
//! what it needs, release the store lock while the network runs, and apply the
//! result afterwards.

use crate::caldav::client::CalDavClient;
use crate::caldav::model::{CalendarComponent, CalendarInfo};
use crate::domain::{
    AppEvent, AppTask, AppTaskStatus, CollectionId, EventId, EventScope, EventTiming, RemoteId,
};
use crate::error::AppError;
use crate::ics;
use chrono::Utc;

use super::model::{Resource, Store};

/// Fetch every collection and replace the in-memory resource map.
///
/// The fetched resources are collected before the map is swapped, so a failed
/// fetch leaves the previous contents untouched.
///
/// # Errors
///
/// Returns the first transport/parse failure; a malformed single resource is
/// logged and skipped rather than aborting the whole load.
pub async fn full_load(store: &mut Store, client: &CalDavClient) -> Result<(), AppError> {
    let calendars: Vec<CalendarInfo> = store.calendars.clone();
    let mut fetched = Vec::with_capacity(calendars.len());
    for calendar in calendars {
        let resources = client.fetch_resources(&calendar.href).await?;
        fetched.push((calendar.href, resources));
    }

    store.resources.clear();
    for (collection, resources) in fetched {
        apply_fetched(store, &collection, resources);
    }
    Ok(())
}

fn apply_fetched(
    store: &mut Store,
    collection: &str,
    fetched: Vec<crate::caldav::xml::MultiGetResource>,
) {
    for resource in fetched {
        let Some(raw) = resource.calendar_data else {
            continue;
        };
        if let Err(error) =
            store.set_resource(&resource.href, collection, resource.etag.as_deref(), &raw)
        {
            eprintln!("Skipping unparseable resource {}: {error}", resource.href);
        }
    }
}

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
    let is_series = master.as_ref().is_some_and(|master| master.rrule.is_some());

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
    let recurring = event.rrule.is_some() || event.recurring_event_id.is_some();
    if !recurring || scope == EventScope::All {
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
            updated.rrule = Some(crate::domain::timing::truncate_before(
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


/// Create one task resource.
///
/// # Errors
///
/// Returns an error for an empty id, a read-only/unknown target collection, or a
/// failed `PUT`.
pub async fn create_task(
    store: &mut Store,
    client: &CalDavClient,
    mut task: AppTask,
) -> Result<AppTask, AppError> {
    let collection = collection_for(
        store,
        CalendarComponent::VTodo,
        task.remote_collection_id.as_ref().map(|id| id.as_str()),
    )?;
    let uid = task.id.as_str().to_string();
    require_uid(&uid)?;
    let href = resource_href(&collection.href, &uid);
    let raw = ics::create_task_resource(&task)?;
    let etag = client.put_resource(&href, &raw, None, true).await?;

    task.remote_id = Some(RemoteId(href.clone()));
    task.remote_collection_id = Some(CollectionId(collection.href.clone()));
    store.set_resource(&href, &collection.href, etag.as_deref(), &raw)?;
    Ok(task)
}

/// Update one task resource, moving it when its collection changed.
///
/// # Errors
///
/// Returns [`AppError::NotFound`] when the task is not in the store,
/// [`AppError::InvalidInput`] for a read-only target, and propagates transport
/// errors.
pub async fn update_task(
    store: &mut Store,
    client: &CalDavClient,
    task: AppTask,
) -> Result<(), AppError> {
    let resource = resource_for_task(store, &task)?;
    ensure_not_ending_a_series(&resource, &task)?;
    let target = match task.remote_collection_id.as_deref() {
        Some(href) => writable_calendar(store, href, CalendarComponent::VTodo)?,
        None => collection_for(store, CalendarComponent::VTodo, None)?,
    };

    let merged = ics::write_task_resource(&resource.raw, &task)?;

    if target.href != resource.collection {
        let href = resource_href(&target.href, task.id.as_str());
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
                .ok_or_else(|| AppError::NotFound(format!("task {}", task.id)))?;
            let merged = ics::write_task_resource(&fresh.raw, &task)?;
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

/// Delete a task resource.
///
/// # Errors
///
/// Propagates transport errors from the delete or component rewrite.
pub async fn delete_task(
    store: &mut Store,
    client: &CalDavClient,
    id: &str,
) -> Result<(), AppError> {
    let Some(resource) = store.resource_for_task(id).cloned() else {
        return Ok(());
    };
    let Some(task) = resource
        .as_tasks()
        .and_then(|tasks| tasks.iter().find(|task| task.id.as_str() == id))
        .cloned()
    else {
        return Ok(());
    };

    if let Some(raw) = ics::delete_task_component(&resource.raw, &task)? {
        let etag = client
            .put_resource(&resource.href, &raw, resource.etag.as_deref(), false)
            .await?;
        store.set_resource(
            &resource.href,
            &resource.collection,
            etag.as_deref().or(resource.etag.as_deref()),
            &raw,
        )?;
    } else {
        client
            .delete_resource(&resource.href, resource.etag.as_deref())
            .await?;
        store.remove_resource(&resource.href);
    }
    Ok(())
}

/// Ids of open tasks whose linked event has already ended.
#[must_use]
pub fn past_due_task_ids(store: &Store) -> Vec<String> {
    let now = Utc::now();
    let events = store.events();
    store
        .tasks()
        .into_iter()
        .filter(|task| !is_done(task))
        .filter(|task| {
            events.iter().any(|event| {
                event
                    .task_id
                    .as_ref()
                    .is_some_and(|id| id.as_str() == task.id.as_str())
                    && EventTiming::from_event(event).is_some_and(|timing| match timing {
                        EventTiming::Timed { end, .. } => end < now,
                        EventTiming::AllDay { .. } => false,
                    })
            })
        })
        .map(|task| task.id.to_string())
        .collect()
}

const fn is_done(task: &AppTask) -> bool {
    matches!(task.status, Some(crate::domain::AppTaskStatus::Done))
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

/// tasks.org completes a recurring task by rolling the series forward; a plain
/// `STATUS:COMPLETED` on a recurring master would end the series for every
/// client instead. Refuse until that roll-forward exists.
fn ensure_not_ending_a_series(resource: &Resource, task: &AppTask) -> Result<(), AppError> {
    if task.status != Some(AppTaskStatus::Done) {
        return Ok(());
    }
    let Some(previous) = resource
        .as_tasks()
        .and_then(|tasks| tasks.iter().find(|candidate| candidate.id == task.id))
    else {
        return Ok(());
    };
    let repeats = previous.rrule.is_some() || task.rrule.is_some();
    if !repeats || previous.status == Some(AppTaskStatus::Done) {
        return Ok(());
    }
    Err(AppError::InvalidInput(format!(
        "task {} repeats; completing it would end the series for every client",
        task.id
    )))
}

fn resource_for_task(store: &Store, task: &AppTask) -> Result<Resource, AppError> {
    task.remote_id
        .as_deref()
        .and_then(|href| store.resources.get(href))
        .or_else(|| store.resource_for_task(task.id.as_str()))
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("task {}", task.id)))
}

async fn refetch_resource(
    client: &CalDavClient,
    store: &Store,
    resource: &Resource,
) -> Result<Option<Resource>, AppError> {
    let fetched = client
        .fetch_hrefs(&resource.collection, std::slice::from_ref(&resource.href))
        .await?;
    let Some(mut fetched) = fetched.into_iter().next() else {
        return Ok(None);
    };
    let Some(raw) = fetched.calendar_data.take() else {
        return Ok(None);
    };
    let mut clone = store.resources.get(&resource.href).cloned();
    if let Some(clone) = clone.as_mut() {
        clone.raw.clone_from(&raw);
        clone.etag.clone_from(&fetched.etag);
    }
    Ok(clone)
}

fn collection_for(
    store: &Store,
    component: CalendarComponent,
    requested: Option<&str>,
) -> Result<CalendarInfo, AppError> {
    if let Some(requested) = requested {
        if let Some(calendar) = store.calendar(requested) {
            if !calendar.writable {
                return Err(AppError::InvalidInput(
                    "the target calendar is read-only".to_string(),
                ));
            }
            if calendar.component != component {
                return Err(AppError::InvalidInput(format!(
                    "the target collection stores {}, not {}",
                    calendar.component, component
                )));
            }
            return Ok(calendar.clone());
        }
    }
    store
        .default_calendar(component, None)
        .cloned()
        .ok_or_else(|| {
            AppError::InvalidInput(format!("no writable {component} collection is configured"))
        })
}

fn writable_calendar(
    store: &Store,
    href: &str,
    component: CalendarComponent,
) -> Result<CalendarInfo, AppError> {
    let calendar = store
        .calendar(href)
        .ok_or_else(|| AppError::NotFound(format!("calendar {href}")))?;
    if !calendar.writable {
        return Err(AppError::InvalidInput(
            "the target calendar is read-only".to_string(),
        ));
    }
    if calendar.component != component {
        return Err(AppError::InvalidInput(format!(
            "the target collection stores {}, not {}",
            calendar.component, component
        )));
    }
    Ok(calendar.clone())
}

fn require_uid(uid: &str) -> Result<(), AppError> {
    if uid.is_empty() || uid.contains('/') {
        return Err(AppError::InvalidInput(
            "an event id must be a non-empty UID without a slash".to_string(),
        ));
    }
    Ok(())
}

fn resource_href(collection: &str, uid: &str) -> String {
    format!("{}/{uid}.ics", collection.trim_end_matches('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_href_normalises_the_collection_trailing_slash() {
        assert_eq!(
            resource_href("/me/cal/", "abc"),
            "/me/cal/abc.ics".to_string()
        );
        assert_eq!(
            resource_href("/me/cal", "abc"),
            "/me/cal/abc.ics".to_string()
        );
    }

    #[test]
    fn uids_with_a_slash_are_rejected() {
        assert!(require_uid("abc").is_ok());
        assert!(require_uid("").is_err());
        assert!(require_uid("a/b").is_err());
    }
}
