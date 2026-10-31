//! Store operations: load/pull from `CalDAV`, and create/update/delete resources.
//!
//! These take the lock-free `Store` and a client so the command layer can copy
//! what it needs, release the store lock while the network runs, and apply the
//! result afterwards.
//!
//! Split by aggregate: [`load`] for the one-shot pull, [`events`] and [`tasks`]
//! for their respective writes. The shared helpers and re-exports live here, so
//! call sites keep using `ops::<operation>`.

mod events;
mod load;
mod tasks;

pub use events::{create_event, delete_event, reschedule_event, update_event, Reschedule};
pub use load::full_load;
pub use tasks::{create_task, delete_task, past_due_task_ids, update_task};

use crate::features::calendar::caldav::client::CalDavClient;
use crate::features::calendar::caldav::model::{CalendarComponent, CalendarInfo};
use crate::error::AppError;

use super::model::{Resource, Store};

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
