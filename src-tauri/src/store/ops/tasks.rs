//! Task writes: create/update/delete plus the recurring-task completion guard.

use chrono::Utc;

use crate::caldav::client::CalDavClient;
use crate::caldav::model::CalendarComponent;
use crate::domain::{AppTask, AppTaskStatus, CollectionId, EventTiming, RemoteId};
use crate::error::AppError;
use crate::ics;

use super::super::model::{Resource, Store};
use super::{collection_for, refetch_resource, require_uid, resource_href, writable_calendar};

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
    matches!(task.status, Some(AppTaskStatus::Done))
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
