//! Store operations: load/pull from `CalDAV`, and create/update/delete resources.
//!
//! Write operations take [`StoreState`] and a [`CalDavClient`]: they lock the
//! store briefly to snapshot and validate inputs, release the lock for every
//! `CalDAV` HTTP request, and re-acquire it briefly to apply the server-confirmed
//! result under a revision/ETag guard.
//!
//! Split by aggregate: [`load`] for the one-shot pull, [`tasks`], [`events`],
//! [`event_delete`], and [`tags`] for their respective writes. The shared helpers
//! and re-exports live here so call sites keep using `ops::<operation>`.

mod event_delete;
mod events;
mod load;
mod tags;
mod tasks;

pub use event_delete::delete_event;
pub use events::{create_event, reschedule_event, update_event, Reschedule};
pub use load::{apply, apply_full_load, fetch_collections, full_load, Fetched, FetchedCollection};
pub use tags::rename_tag;
pub use tasks::{create_task, delete_task, past_due_task_ids, update_task};

use crate::error::AppError;
use crate::features::calendar::caldav::client::CalDavClient;
use crate::features::calendar::caldav::model::{CalendarComponent, CalendarInfo};

use super::model::{parse_items, Resource, Store};

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

pub(super) async fn refetch_resource(
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

pub(super) fn apply_resource_update(
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

pub(super) fn apply_resource_removal(
    store: &mut Store,
    href: &str,
    base_revision: u64,
    initial_etag: Option<&str>,
) {
    if store.revision() == base_revision
        || store
            .resources
            .get(href)
            .is_some_and(|current| current.etag.as_deref() == initial_etag)
    {
        store.remove_resource(href);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_resource_move(
    store: &mut Store,
    old_href: &str,
    new_href: &str,
    target_collection: &str,
    base_revision: u64,
    initial_etag: Option<&str>,
    new_etag: Option<&str>,
    raw: &str,
) -> Result<(), AppError> {
    let unchanged = store.revision() == base_revision;
    if unchanged {
        store.remove_resource(old_href);
        return store.set_resource(new_href, target_collection, new_etag, raw);
    }
    apply_resource_removal(store, old_href, base_revision, initial_etag);
    if store.calendar(target_collection).is_some() && !store.resources.contains_key(new_href) {
        return store.set_resource(new_href, target_collection, new_etag, raw);
    }
    Ok(())
}

pub(super) fn ensure_source_writable(
    store: &Store,
    collection_href: &str,
) -> Result<(), AppError> {
    if let Some(calendar) = store.calendar(collection_href) {
        if !calendar.writable {
            return Err(AppError::InvalidInput(
                "the source calendar is read-only".to_string(),
            ));
        }
    }
    Ok(())
}

pub(super) fn collection_for(
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

pub(super) fn writable_calendar(
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

pub(super) fn require_uid(uid: &str) -> Result<(), AppError> {
    if uid.is_empty() || uid.contains('/') {
        return Err(AppError::InvalidInput(
            "an event id must be a non-empty UID without a slash".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn resource_href(collection: &str, uid: &str) -> String {
    format!(
        "{}/{}.ics",
        collection.trim_end_matches('/'),
        encode_path_segment(uid)
    )
}

/// Percent-encode a filename segment using RFC 3986 unreserved characters
/// (`A-Za-z0-9-._~`), matching Radicale's `urllib.parse.quote` `<href>` output
/// (including `@` → `%40` for `uid@domain` identifiers).
fn encode_path_segment(segment: &str) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(segment.len());
    for &byte in segment.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
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
    fn resource_href_percent_encodes_path_unsafe_characters_in_uid() {
        assert_eq!(
            resource_href("/me/cal/", "event@example.com"),
            "/me/cal/event%40example.com.ics"
        );
        assert_eq!(
            resource_href("/me/cal/", "a#b?c 100%"),
            "/me/cal/a%23b%3Fc%20100%25.ics"
        );
        assert_eq!(
            resource_href("/me/cal/", "café"),
            "/me/cal/caf%C3%A9.ics"
        );
    }

    #[test]
    fn uids_with_a_slash_are_rejected() {
        assert!(require_uid("abc").is_ok());
        assert!(require_uid("").is_err());
        assert!(require_uid("a/b").is_err());
    }
}
