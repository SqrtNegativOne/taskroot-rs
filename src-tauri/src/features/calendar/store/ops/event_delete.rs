//! Scoped event deletion (whole series, single occurrence, or following).

use crate::error::AppError;
use crate::features::calendar::caldav::client::CalDavClient;
use crate::features::calendar::domain::{AppEvent, CalendarDateTime, EventScope};
use crate::features::calendar::ics;

use super::super::model::{Resource, Store};
use super::super::state::StoreState;
use super::events::selected_occurrence;
use super::{
    apply_resource_removal, apply_resource_update, ensure_source_writable, UpdateGuard,
};

enum DeletePlan {
    PutRewritten {
        resource: Resource,
        raw: String,
        base_revision: u64,
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
        } => {
            let etag = client
                .put_resource(&resource.href, &raw, resource.etag.as_deref(), false)
                .await?;
            let final_etag = etag.or_else(|| resource.etag.clone());
            state
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
                .await
        }
        DeletePlan::DeleteWhole {
            resource,
            base_revision,
        } => {
            client
                .delete_resource(&resource.href, resource.etag.as_deref())
                .await?;
            state
                .write(|store| {
                    apply_resource_removal(
                        store,
                        &resource.href,
                        base_revision,
                        resource.etag.as_deref(),
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
    let master = master_event(store, &event);
    let is_series = master.as_ref().is_some_and(AppEvent::is_recurring);

    if scope == EventScope::All || !is_series {
        if let Some(master) = master {
            if let Some(raw) = ics::delete_event_component(&resource.raw, &master)? {
                return Ok(Some(DeletePlan::PutRewritten {
                    resource,
                    raw,
                    base_revision,
                }));
            }
        }
        return Ok(Some(DeletePlan::DeleteWhole {
            resource,
            base_revision,
        }));
    }

    let occurrence = selected_occurrence(&event, occurrence_start);
    let Some(master) = master else {
        return Err(AppError::NotFound(format!("series of event {id}")));
    };
    let mut raw = resource.raw.clone();

    if scope == EventScope::This && event.recurring_event_id.is_some() {
        let Some(remaining) = ics::delete_event_component(&raw, &event)? else {
            return Ok(Some(DeletePlan::DeleteWhole {
                resource,
                base_revision,
            }));
        };
        raw = remaining;
    }

    if scope == EventScope::Following {
        for victim in exceptions_from(&resource, occurrence) {
            if let Some(remaining) = ics::delete_event_component(&raw, &victim)? {
                raw = remaining;
            }
        }
    }

    let updated = scoped_master(&master, scope, occurrence)?;
    let raw = ics::write_event_resource(&raw, &updated)?;
    Ok(Some(DeletePlan::PutRewritten {
        resource,
        raw,
        base_revision,
    }))
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

fn master_event(store: &Store, event: &AppEvent) -> Option<AppEvent> {
    event
        .recurring_event_id
        .as_deref()
        .map_or_else(|| Some(event.clone()), |master_id| store.event(master_id).cloned())
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
