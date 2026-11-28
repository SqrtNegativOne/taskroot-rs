//! Scoped event deletion (whole series, single occurrence, or following).

use crate::error::AppError;
use crate::features::calendar::caldav::client::CalDavClient;
use crate::features::calendar::domain::{AppEvent, CalendarDateTime, EventScope};
use crate::features::calendar::ics;

use super::super::model::{Resource, Store};
use super::super::state::StoreState;
use super::events::selected_occurrence;
use super::{
    apply_resource_removal, delete_with_conflict_retry, ensure_source_writable,
    put_with_conflict_retry, resolve_master, UpdateGuard,
};

enum DeletePlan {
    PutRewritten {
        resource: Resource,
        raw: String,
        base_revision: u64,
        master_id: Option<String>,
    },
    DeleteWhole {
        resource: Resource,
        base_revision: u64,
    },
}

/// Delete a recurring event's occurrences according to `scope`, without holding
/// the store lock during network I/O.
///
/// [`EventScope::All`] removes the whole series, [`EventScope::This`] excludes
/// only `occurrence_start` (and drops the exception row when that is what was
/// selected), and [`EventScope::Following`] ends the rule just before it.
///
/// # Errors
///
/// Propagates transport errors from the delete or the rewrite.
pub async fn delete_event(
    state: &StoreState,
    client: &CalDavClient,
    id: &str,
    scope: EventScope,
    occurrence_start: Option<CalendarDateTime>,
) -> Result<(), AppError> {
    let plan = state
        .read(|store| plan_delete(store, id, scope, occurrence_start))
        .await?;

    let Some(plan) = plan else {
        return Ok(());
    };

    match plan {
        DeletePlan::PutRewritten {
            resource,
            raw,
            base_revision,
            master_id,
        } => {
            put_with_conflict_retry(
                state,
                client,
                &resource,
                base_revision,
                raw,
                &format!("event {id}"),
                |fresh| {
                    rewrite_delete_in_resource(
                        fresh,
                        id,
                        master_id.as_deref(),
                        scope,
                        occurrence_start,
                    )?
                    .ok_or_else(|| AppError::NotFound(format!("event {id}")))
                },
            )
            .await
        }
        DeletePlan::DeleteWhole {
            resource,
            base_revision,
        } => {
            let deleted_etag = delete_with_conflict_retry(client, &resource).await?;
            state
                .write(|store| {
                    apply_resource_removal(
                        store,
                        &resource.href,
                        UpdateGuard {
                            base_revision,
                            initial_etag: resource.etag.as_deref(),
                            committed_from_etag: deleted_etag.as_deref(),
                        },
                    );
                })
                .await;
            Ok(())
        }
    }
}

fn plan_delete(
    store: &Store,
    id: &str,
    scope: EventScope,
    occurrence_start: Option<CalendarDateTime>,
) -> Result<Option<DeletePlan>, AppError> {
    let Some((resource, event)) = series_resource(store, id) else {
        return Ok(None);
    };
    ensure_source_writable(store, &resource.collection)?;
    let base_revision = store.revision();
    let master = resolve_master(store, &event);
    let master_id = master.as_ref().map(|item| item.id.to_string());

    match rewrite_for_delete(&resource, &event, master.as_ref(), scope, occurrence_start)? {
        Some(raw) => Ok(Some(DeletePlan::PutRewritten {
            resource,
            raw,
            base_revision,
            master_id,
        })),
        None => Ok(Some(DeletePlan::DeleteWhole {
            resource,
            base_revision,
        })),
    }
}

fn rewrite_delete_in_resource(
    resource: &Resource,
    id: &str,
    master_id: Option<&str>,
    scope: EventScope,
    occurrence_start: Option<CalendarDateTime>,
) -> Result<Option<String>, AppError> {
    let events = resource.as_events().unwrap_or_default();
    let event = events
        .iter()
        .find(|candidate| candidate.id.as_str() == id)
        .or_else(|| {
            master_id
                .and_then(|mid| events.iter().find(|candidate| candidate.id.as_str() == mid))
        })
        .ok_or_else(|| AppError::NotFound(format!("event {id}")))?;
    let master = event
        .recurring_event_id
        .as_deref()
        .map(String::as_str)
        .or(master_id)
        .and_then(|mid| events.iter().find(|candidate| candidate.id.as_str() == mid))
        .or(Some(event));
    rewrite_for_delete(resource, event, master, scope, occurrence_start)
}

fn rewrite_for_delete(
    resource: &Resource,
    event: &AppEvent,
    master: Option<&AppEvent>,
    scope: EventScope,
    occurrence_start: Option<CalendarDateTime>,
) -> Result<Option<String>, AppError> {
    let is_series = master.is_some_and(AppEvent::is_recurring);
    if scope == EventScope::All || !is_series {
        if let Some(master) = master {
            if let Some(raw) = ics::delete_event_component(&resource.raw, master)? {
                return Ok(Some(raw));
            }
        }
        return Ok(None);
    }

    let occurrence = selected_occurrence(event, occurrence_start);
    let Some(master) = master else {
        return Err(AppError::NotFound(format!("series of event {}", event.id)));
    };
    let mut raw = resource.raw.clone();

    if scope == EventScope::This && event.recurring_event_id.is_some() {
        let Some(remaining) = ics::delete_event_component(&raw, event)? else {
            return Ok(None);
        };
        raw = remaining;
    }

    if scope == EventScope::Following {
        for victim in exceptions_from(resource, occurrence) {
            if let Some(remaining) = ics::delete_event_component(&raw, &victim)? {
                raw = remaining;
            }
        }
    }

    let updated = scoped_master(master, scope, occurrence)?;
    Ok(Some(ics::write_event_resource(&raw, &updated)?))
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

fn exceptions_from(resource: &Resource, occurrence: CalendarDateTime) -> Vec<AppEvent> {
    resource
        .as_events()
        .unwrap_or_default()
        .iter()
        .filter(|event| event.recurring_event_id.is_some())
        .filter(|event| event.original_start_time.is_some_and(|start| start >= occurrence))
        .cloned()
        .collect()
}

fn scoped_master(
    master: &AppEvent,
    scope: EventScope,
    occurrence: CalendarDateTime,
) -> Result<AppEvent, AppError> {
    let mut updated = master.clone();
    if scope == EventScope::Following {
        if let Some(rrule) = master.rrule.as_deref() {
            updated.rrule = Some(crate::features::calendar::domain::timing::truncate_before(
                rrule,
                occurrence,
                master.timezone.as_deref(),
            )?);
        }
        return Ok(updated);
    }
    let mut exdates = master.exdates.clone().unwrap_or_default();
    if !exdates.contains(&occurrence) {
        exdates.push(occurrence);
    }
    updated.exdates = Some(exdates);
    Ok(updated)
}
