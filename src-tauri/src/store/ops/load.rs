//! The one-shot pull: fetch every collection and replace the resource map.

use crate::caldav::client::CalDavClient;
use crate::caldav::model::CalendarInfo;
use crate::error::AppError;

use super::super::model::Store;

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
