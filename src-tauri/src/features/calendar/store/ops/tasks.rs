//! Task writes: create/update/delete plus the recurring-task completion guard.

use chrono::Utc;

use crate::error::AppError;
use crate::features::calendar::caldav::client::CalDavClient;
use crate::features::calendar::caldav::model::CalendarComponent;
use crate::features::calendar::domain::{
    apply_task_patches, AppTask, AppTaskStatus, CollectionId, EventTiming, RemoteId, TaskPatch,
};
use crate::features::calendar::ics;

use super::super::model::{Resource, Store};
use super::super::state::StoreState;
use super::{
    apply_resource_create, apply_resource_move, apply_resource_removal, apply_resource_update,
    collection_for, ensure_source_writable, refetch_resource, require_uid, resource_href,
    writable_calendar, UpdateGuard,
};

/// Create one task resource without holding the store lock during network I/O.
///
/// # Errors
///
/// Returns an error for an empty id, a read-only/unknown target collection, or a
/// failed `PUT`.
pub async fn create_task(
    state: &StoreState,
    client: &CalDavClient,
    mut task: AppTask,
) -> Result<AppTask, AppError> {
    let (collection, href, raw, base_revision) = state
        .read(|store| {
            let collection = collection_for(
                store,
                CalendarComponent::VTodo,
                task.remote_collection_id.as_ref().map(|id| id.as_str()),
            )?;
            let uid = task.id.as_str();
            require_uid(uid)?;
            let href = resource_href(&collection.href, uid);
            let raw = ics::create_task_resource(&task)?;
            Ok::<_, AppError>((collection, href, raw, store.revision()))
        })
        .await?;

    let etag = client.put_resource(&href, &raw, None, true).await?;

    task.remote_id = Some(RemoteId(href.clone()));
    task.remote_collection_id = Some(CollectionId(collection.href.clone()));
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
    Ok(task)
}

/// Apply field `patches` to one task, moving it when its collection changed.
///
/// On `412 Conflict`, refetches the fresh server resource, applies only
/// `patches` onto the freshly parsed task, re-validates invariants, and retries
/// the `PUT` once so concurrent edits to other fields survive.
///
/// # Errors
///
/// Returns [`AppError::NotFound`] when the task is not in the store,
/// [`AppError::InvalidInput`] for a read-only target or recurring completion,
/// and propagates transport errors.
pub async fn update_task(
    state: &StoreState,
    client: &CalDavClient,
    id: &str,
    patches: &[TaskPatch],
) -> Result<(), AppError> {
    let (resource, target, merged, base_revision) = state
        .read(|store| {
            let resource = store
                .resource_for_task(id)
                .cloned()
                .ok_or_else(|| AppError::NotFound(format!("task {id}")))?;
            let mut task = find_task_in(&resource, id)?.clone();
            apply_task_patches(&mut task, patches);
            ensure_not_ending_a_series(&resource, &task)?;
            let target = match task.remote_collection_id.as_deref() {
                Some(href) => writable_calendar(store, href, CalendarComponent::VTodo)?,
                None => collection_for(store, CalendarComponent::VTodo, None)?,
            };
            if target.href != resource.collection {
                ensure_source_writable(store, &resource.collection)?;
            }
            let merged = ics::write_task_resource(&resource.raw, &task)?;
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
        put_task_with_retry(client, &resource, id, patches, merged).await?;

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

async fn put_task_with_retry(
    client: &CalDavClient,
    resource: &Resource,
    id: &str,
    patches: &[TaskPatch],
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
                .ok_or_else(|| AppError::NotFound(format!("task {id}")))?;
            let mut fresh_task = find_task_in(&fresh, id)?.clone();
            apply_task_patches(&mut fresh_task, patches);
            ensure_not_ending_a_series(&fresh, &fresh_task)?;
            let retried = ics::write_task_resource(&fresh.raw, &fresh_task)?;
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

/// Delete a task resource without holding the store lock during network I/O.
///
/// # Errors
///
/// Propagates transport errors from the delete or component rewrite.
pub async fn delete_task(
    state: &StoreState,
    client: &CalDavClient,
    id: &str,
) -> Result<(), AppError> {
    let plan = state
        .read(|store| {
            let Some(resource) = store.resource_for_task(id).cloned() else {
                return Ok::<_, AppError>(None);
            };
            ensure_source_writable(store, &resource.collection)?;
            let Some(task) = resource
                .as_tasks()
                .and_then(|tasks| tasks.iter().find(|task| task.id.as_str() == id))
                .cloned()
            else {
                return Ok(None);
            };
            let rewritten = ics::delete_task_component(&resource.raw, &task)?;
            Ok(Some((resource, rewritten, store.revision())))
        })
        .await?;

    let Some((resource, rewritten, base_revision)) = plan else {
        return Ok(());
    };

    if let Some(raw) = rewritten {
        let etag = client
            .put_resource(&resource.href, &raw, resource.etag.as_deref(), false)
            .await?;
        let final_etag = etag.or_else(|| resource.etag.clone());
        return state
            .write(|store| {
                apply_resource_update(
                    store,
                    &resource.href,
                    &resource.collection,
                    UpdateGuard {
                        base_revision,
                        initial_etag: resource.etag.as_deref(),
                        committed_from_etag: resource.etag.as_deref(),
                    },
                    final_etag.as_deref(),
                    &raw,
                )
            })
            .await;
    }

    client
        .delete_resource(&resource.href, resource.etag.as_deref())
        .await?;
    state
        .write(|store| {
            apply_resource_removal(store, &resource.href, base_revision, resource.etag.as_deref());
        })
        .await;
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

fn find_task_in<'a>(resource: &'a Resource, id: &str) -> Result<&'a AppTask, AppError> {
    resource
        .as_tasks()
        .and_then(|tasks| tasks.iter().find(|task| task.id.as_str() == id))
        .ok_or_else(|| AppError::NotFound(format!("task {id}")))
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
