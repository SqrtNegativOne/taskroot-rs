//! The one-shot pull: fetch every collection and replace the resource map.

use crate::features::calendar::caldav::client::CalDavClient;
use crate::features::calendar::caldav::model::CalendarInfo;
use crate::error::AppError;

use super::super::model::Store;

/// One collection's fetched resources, held outside the store lock until the
/// whole load succeeded.
pub struct FetchedCollection {
    pub href: String,
    pub resources: Vec<crate::features::calendar::caldav::xml::MultiGetResource>,
}

/// Fetch every collection without touching the store.
///
/// # Errors
///
/// Returns the first transport/parse failure; a failed fetch leaves the caller's
/// store untouched because nothing is applied yet.
pub async fn fetch_collections(
    client: &CalDavClient,
    calendars: &[CalendarInfo],
) -> Result<Vec<FetchedCollection>, AppError> {
    let mut fetched = Vec::with_capacity(calendars.len());
    for calendar in calendars {
        let resources = client.fetch_resources(&calendar.href).await?;
        fetched.push(FetchedCollection {
            href: calendar.href.clone(),
            resources,
        });
    }
    Ok(fetched)
}

/// Replace the store contents with a fully fetched set of collections.
pub fn apply_full_load(store: &mut Store, fetched: Vec<FetchedCollection>) {
    store.clear_resources();
    for collection in fetched {
        apply_fetched(store, &collection.href, collection.resources);
    }
}

/// Fetch every collection and replace the in-memory resource map.
///
/// The fetched resources are collected before the map is swapped, so a failed
/// fetch leaves the previous contents untouched. Callers that hold the store
/// behind a lock should call [`fetch_collections`] first and [`apply_full_load`]
/// afterwards, so the lock is not held during network I/O.
///
/// # Errors
///
/// Returns the first transport/parse failure; a malformed single resource is
/// logged and skipped rather than aborting the whole load.
pub async fn full_load(store: &mut Store, client: &CalDavClient) -> Result<(), AppError> {
    let fetched = fetch_collections(client, &store.calendars).await?;
    apply_full_load(store, fetched);
    Ok(())
}

fn apply_fetched(
    store: &mut Store,
    collection: &str,
    fetched: Vec<crate::features::calendar::caldav::xml::MultiGetResource>,
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
