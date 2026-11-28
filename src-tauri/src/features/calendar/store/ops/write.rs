//! Shared conditional-write, `412 Conflict` retry, move rollback, and series
//! master resolution helpers for [`super`].

use crate::error::AppError;
use crate::features::calendar::caldav::client::CalDavClient;
use crate::features::calendar::domain::AppEvent;

use super::super::model::{parse_items, Resource, Store};
use super::super::state::StoreState;

/// Guard for applying an in-flight `PUT` back into [`Store`].
///
/// If nothing mutated the store while the request ran (`store.revision() ==
/// base_revision`), the write applies unconditionally. Otherwise it applies only
/// when the resource's current `ETag` still matches `initial_etag` (what we read
/// before the write) or `committed_from_etag` (what a `412` retry matched on the
/// server), so a newer concurrent write or refresh is never overwritten while an
/// older `full_load` snapshot is superseded.
#[derive(Clone, Copy)]
pub(super) struct UpdateGuard<'a> {
    pub(super) base_revision: u64,
    pub(super) initial_etag: Option<&'a str>,
    pub(super) committed_from_etag: Option<&'a str>,
}

/// Resolve the series master for `event`: returns `event` itself when it is
/// already a master (or non-recurring) and looks up `recurring_event_id` when
/// `event` is an exception row.
pub(super) fn resolve_master(store: &Store, event: &AppEvent) -> Option<AppEvent> {
    event
        .recurring_event_id
        .as_deref()
        .map_or_else(|| Some(event.clone()), |master_id| store.event(master_id).cloned())
}

async fn refetch_resource(
    client: &CalDavClient,
    collection: &str,
    href: &str,
) -> Result<Option<Resource>, AppError> {
    let fetched = client
        .fetch_hrefs(collection, &[href.to_string()])
        .await?;
    let Some(mut fetched) = fetched.into_iter().next() else {
        return Ok(None);
    };
    let Some(raw) = fetched.calendar_data.take() else {
        return Ok(None);
    };
    let items = parse_items(&raw, href, collection)?;
    Ok(Some(Resource {
        href: href.to_string(),
        collection: collection.to_string(),
        etag: fetched.etag,
        raw,
        items,
    }))
}

/// `PUT` an updated resource, retrying once on `412 Conflict` by refetching
/// the server's current resource and re-running `build_raw`, then apply the
/// server-confirmed result to [`StoreState`] under [`UpdateGuard`].
pub(super) async fn put_with_conflict_retry<F>(
    state: &StoreState,
    client: &CalDavClient,
    resource: &Resource,
    base_revision: u64,
    initial_raw: String,
    not_found_label: &str,
    build_raw: F,
) -> Result<(), AppError>
where
    F: Fn(&Resource) -> Result<String, AppError>,
{
    let (final_raw, final_etag, committed_from) = match client
        .put_resource(&resource.href, &initial_raw, resource.etag.as_deref(), false)
        .await
    {
        Ok(etag) => (
            initial_raw,
            etag.or_else(|| resource.etag.clone()),
            resource.etag.clone(),
        ),
        Err(AppError::Conflict(_)) => {
            let fresh = refetch_resource(client, &resource.collection, &resource.href)
                .await?
                .ok_or_else(|| AppError::NotFound(not_found_label.to_string()))?;
            let retried = build_raw(&fresh)?;
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

/// `DELETE` a resource, retrying once on `412 Conflict` with the server's fresh
/// `ETag`. Returns the `ETag` that was deleted (`None` if the resource was
/// already absent on refetch).
pub(super) async fn delete_with_conflict_retry(
    client: &CalDavClient,
    resource: &Resource,
) -> Result<Option<String>, AppError> {
    match client
        .delete_resource(&resource.href, resource.etag.as_deref())
        .await
    {
        Ok(()) => Ok(resource.etag.clone()),
        Err(AppError::Conflict(_)) => {
            let Some(fresh) =
                refetch_resource(client, &resource.collection, &resource.href).await?
            else {
                return Ok(None);
            };
            client
                .delete_resource(&resource.href, fresh.etag.as_deref())
                .await?;
            Ok(fresh.etag)
        }
        Err(error) => Err(error),
    }
}

/// Move a resource across collections: `PUT` destination (`If-None-Match: *`),
/// then `DELETE` source (retrying `412` once). If source deletion fails with a
/// non-404 error, attempts a compensating `DELETE` of `new_href` so no remote
/// duplicate is left behind.
pub(super) async fn move_resource(
    state: &StoreState,
    client: &CalDavClient,
    resource: &Resource,
    target_collection: &str,
    new_href: &str,
    merged: &str,
    base_revision: u64,
) -> Result<(), AppError> {
    let new_etag = client.put_resource(new_href, merged, None, true).await?;
    let deleted_etag = match delete_with_conflict_retry(client, resource).await {
        Ok(etag) => etag,
        Err(error) => {
            if let Err(rollback_err) = client.delete_resource(new_href, new_etag.as_deref()).await {
                eprintln!(
                    "Failed to roll back moved resource {new_href} after source delete failed: {rollback_err}"
                );
            }
            return Err(error);
        }
    };
    state
        .write(|store| {
            apply_resource_move(
                store,
                &resource.href,
                new_href,
                target_collection,
                UpdateGuard {
                    base_revision,
                    initial_etag: resource.etag.as_deref(),
                    committed_from_etag: deleted_etag.as_deref(),
                },
                new_etag.as_deref(),
                merged,
            )
        })
        .await
}

pub(super) fn apply_resource_create(
    store: &mut Store,
    href: &str,
    collection: &str,
    base_revision: u64,
    etag: Option<&str>,
    raw: &str,
) -> Result<(), AppError> {
    if store.revision() == base_revision {
        return store.set_resource(href, collection, etag, raw);
    }
    if store.calendar(collection).is_some() && !store.resources.contains_key(href) {
        return store.set_resource(href, collection, etag, raw);
    }
    Ok(())
}

fn apply_resource_update(
    store: &mut Store,
    href: &str,
    collection: &str,
    guard: UpdateGuard<'_>,
    final_etag: Option<&str>,
    raw: &str,
) -> Result<(), AppError> {
    if store.revision() == guard.base_revision {
        return store.set_resource(href, collection, final_etag, raw);
    }
    let Some(current) = store.resources.get(href) else {
        return Ok(());
    };
    let current_etag = current.etag.as_deref();
    if current_etag == guard.initial_etag || current_etag == guard.committed_from_etag {
        return store.set_resource(href, collection, final_etag, raw);
    }
    Ok(())
}

pub(super) fn apply_resource_removal(store: &mut Store, href: &str, guard: UpdateGuard<'_>) {
    if store.revision() == guard.base_revision
        || store.resources.get(href).is_some_and(|current| {
            let etag = current.etag.as_deref();
            etag == guard.initial_etag || etag == guard.committed_from_etag
        })
    {
        store.remove_resource(href);
    }
}

fn apply_resource_move(
    store: &mut Store,
    old_href: &str,
    new_href: &str,
    target_collection: &str,
    guard: UpdateGuard<'_>,
    new_etag: Option<&str>,
    raw: &str,
) -> Result<(), AppError> {
    if store.revision() == guard.base_revision {
        store.remove_resource(old_href);
        return store.set_resource(new_href, target_collection, new_etag, raw);
    }
    apply_resource_removal(store, old_href, guard);
    if store.calendar(target_collection).is_some() && !store.resources.contains_key(new_href) {
        return store.set_resource(new_href, target_collection, new_etag, raw);
    }
    Ok(())
}
