//! Store operations: load/pull from `CalDAV`, and create/update/delete resources.
//!
//! These take the lock-free `Store` and a client so the command layer can copy
//! what it needs, release the store lock while the network runs, and apply the
//! result afterwards.

use crate::caldav::client::CalDavClient;
use crate::caldav::model::{CalendarComponent, CalendarInfo};
use crate::domain::{AppEvent, AppTask, CollectionId, EventTiming, RemoteId};
use crate::error::AppError;
use crate::ics;
use chrono::Utc;
use std::collections::BTreeSet;

use super::model::{Resource, Store};

/// Fetch every collection and replace the in-memory resource map.
///
/// # Errors
///
/// Returns the first transport/parse failure; a malformed single resource is
/// logged and skipped rather than aborting the whole load.
pub async fn full_load(store: &mut Store, client: &CalDavClient) -> Result<(), AppError> {
    let calendars: Vec<CalendarInfo> = store.calendars.clone();
    store.resources.clear();
    store.tokens.clear();
    for calendar in calendars {
        load_collection(store, client, &calendar).await?;
    }
    store.mark_synced();
    store.write_snapshot();
    Ok(())
}

/// Pull every changed resource, using per-collection sync tokens.
///
/// # Errors
///
/// Returns a transport failure other than the token-invalid cases that trigger a
/// full collection reload.
pub async fn pull(store: &mut Store, client: &CalDavClient) -> Result<(), AppError> {
    let calendars: Vec<CalendarInfo> = store.calendars.clone();
    let mut fallbacks: Vec<CalendarInfo> = Vec::new();

    for calendar in calendars {
        let token = store.tokens.get(&calendar.href).cloned();
        match client
            .sync_collection(&calendar.href, token.as_deref())
            .await
        {
            Ok(page) => {
                let removed: Vec<String> = page
                    .changes
                    .iter()
                    .filter(|change| change.is_removed())
                    .map(|change| change.href.clone())
                    .collect();
                let live: Vec<String> = page
                    .changes
                    .iter()
                    .filter(|change| !change.is_removed())
                    .map(|change| change.href.clone())
                    .collect();

                for href in removed {
                    store.remove_resource(&href);
                }
                apply_fetched(store, &calendar.href, client.fetch_hrefs(&calendar.href, &live).await?);
                match page.token {
                    Some(token) => {
                        store.tokens.insert(calendar.href.clone(), token);
                    }
                    None => {
                        store.tokens.remove(&calendar.href);
                    }
                }
            }
            // 403/410/501 all surface as Sync/Gone; a full collection reload is
            // the documented recovery and is idempotent.
            Err(AppError::Gone(_) | AppError::Sync(_)) => fallbacks.push(calendar),
            Err(error) => return Err(error),
        }
    }

    for calendar in fallbacks {
        store.tokens.remove(&calendar.href);
        load_collection(store, client, &calendar).await?;
    }

    store.mark_synced();
    store.write_snapshot();
    Ok(())
}

async fn load_collection(
    store: &mut Store,
    client: &CalDavClient,
    calendar: &CalendarInfo,
) -> Result<(), AppError> {
    let fetched = client.fetch_resources(&calendar.href).await?;
    let live: BTreeSet<&str> = fetched
        .iter()
        .filter(|resource| resource.calendar_data.is_some())
        .map(|resource| resource.href.as_str())
        .collect();
    let stale: Vec<String> = store
        .resources
        .iter()
        .filter(|(_, resource)| {
            resource.collection == calendar.href && !live.contains(resource.href.as_str())
        })
        .map(|(href, _)| href.clone())
        .collect();
    for href in stale {
        store.remove_resource(&href);
    }
    apply_fetched(store, &calendar.href, fetched);
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
        if let Err(error) = store.set_resource(
            &resource.href,
            collection,
            resource.etag.as_deref(),
            &raw,
        ) {
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
    store.write_snapshot();
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
        store.write_snapshot();
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

    store.set_resource(&resource.href, &resource.collection, etag.as_deref(), &merged)?;
    store.write_snapshot();
    Ok(())
}

/// Delete an event. A master deletes its whole resource; an exception removes
/// only its component.
///
/// # Errors
///
/// Propagates transport errors from the delete or the exception rewrite.
pub async fn delete_event(
    store: &mut Store,
    client: &CalDavClient,
    id: &str,
) -> Result<(), AppError> {
    let Some(resource) = store.resource_for_event(id).cloned() else {
        return Ok(());
    };
    let Some(event) = resource
        .as_events()
        .and_then(|events| events.iter().find(|event| event.id.as_str() == id))
        .cloned()
    else {
        return Ok(());
    };

    if event.recurring_event_id.is_none() {
        client
            .delete_resource(&resource.href, resource.etag.as_deref())
            .await?;
        store.remove_resource(&resource.href);
        store.write_snapshot();
        return Ok(());
    }

    if let Some(raw) = ics::delete_event_component(&resource.raw, &event)? {
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
    store.write_snapshot();
    Ok(())
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
    store.write_snapshot();
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
        store.write_snapshot();
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

    store.set_resource(&resource.href, &resource.collection, etag.as_deref(), &merged)?;
    store.write_snapshot();
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
    store.write_snapshot();
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
                event.task_id.as_ref().is_some_and(|id| id.as_str() == task.id.as_str())
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
            AppError::InvalidInput(format!(
                "no writable {component} collection is configured"
            ))
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
