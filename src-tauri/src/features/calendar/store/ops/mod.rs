//! Store operations: load/pull from `CalDAV`, and create/update/delete resources.
//!
//! Write operations take [`StoreState`] and a [`CalDavClient`]: they lock the
//! store briefly to snapshot and validate inputs, release the lock for every
//! `CalDAV` HTTP request, and re-acquire it briefly to apply the server-confirmed
//! result under a revision/ETag guard.
//!
//! Split by aggregate: [`load`] for the one-shot pull, [`tasks`], [`events`],
//! [`event_delete`], and [`tags`] for their respective writes, and [`write`]
//! for shared conditional-write retry and cross-collection move rollback.

mod event_delete;
mod events;
mod load;
mod tags;
mod tasks;
mod write;

pub use event_delete::delete_event;
pub use events::{create_event, reschedule_event, update_event, Reschedule};
pub use load::{apply, apply_full_load, fetch_collections, full_load, Fetched, FetchedCollection};
pub use tags::rename_tag;
pub use tasks::{create_task, delete_task, past_due_task_ids, update_task};

use crate::error::AppError;
use crate::features::calendar::caldav::model::{CalendarComponent, CalendarInfo};

use super::model::Store;
use write::{
    apply_resource_create, apply_resource_removal, delete_with_conflict_retry, move_resource,
    put_with_conflict_retry, resolve_master, UpdateGuard,
};

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
