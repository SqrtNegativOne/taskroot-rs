//! Drains the offline sync queue into Google, one item at a time.

use crate::domain::{AppEvent, AppTask, RemoteId};
use crate::sync::queue::SyncQueue;
use crate::sync::types::{SyncAction, SyncItemData, SyncQueueItem};
use sqlx::SqlitePool;
use std::sync::Arc;

/// Pushes every queued item, stopping at the first failure to keep ordering.
///
/// Returns the failure message so the caller can surface it; a queue that is
/// stuck on one item is otherwise invisible to the user.
#[must_use]
pub(super) async fn flush_queue(pool: &SqlitePool, access_token: &str) -> Option<String> {
    let queue = SyncQueue::new(Arc::new(pool.clone()));
    while let Ok(Some((queue_id, item))) = queue.peek().await {
        match push_item(pool, item, access_token).await {
            Ok(()) => {
                let _ = queue.remove(queue_id).await;
            }
            Err(message) => return Some(message),
        }
    }
    None
}

async fn push_item(pool: &SqlitePool, item: SyncQueueItem, access_token: &str) -> Result<(), String> {
    let SyncQueueItem {
        action,
        item: data,
        calendar_id,
        destination_calendar_id,
        ..
    } = item;

    match data {
        SyncItemData::Task(task) => push_task(pool, action, task, access_token).await,
        SyncItemData::Event(event) => {
            push_event(
                pool,
                action,
                calendar_id,
                destination_calendar_id,
                event,
                access_token,
            )
            .await
        }
    }
}

async fn push_task(
    pool: &SqlitePool,
    action: SyncAction,
    mut task: AppTask,
    access_token: &str,
) -> Result<(), String> {
    if action == SyncAction::Delete {
        let Some(remote_id) = task.remote_id.clone() else {
            return Ok(());
        };
        return crate::apis::google_tasks::delete(&remote_id, access_token)
            .await
            .map_err(|e| e.to_string());
    }

    match crate::apis::google_tasks::publish(&task, access_token).await {
        Ok(remote_id) => {
            task.remote_id = Some(RemoteId(remote_id));
            task.dirty = Some(false);
            crate::db::upsert_task(pool, task)
                .await
                .map_err(|e| e.to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}

async fn push_event(
    pool: &SqlitePool,
    action: SyncAction,
    source_calendar_id: Option<String>,
    destination_calendar_id: Option<String>,
    event: AppEvent,
    access_token: &str,
) -> Result<(), String> {
    match action {
        SyncAction::Delete => {
            let Some(remote_id) = event.remote_id.clone() else {
                return Ok(());
            };
            let calendar = source_calendar_id
                .as_deref()
                .or_else(|| event.remote_collection_id.as_deref().map(String::as_str));
            crate::apis::google_calendar::delete(&remote_id, calendar, access_token)
                .await
                .map_err(|e| e.to_string())
        }
        SyncAction::Move => {
            push_event_move(
                pool,
                source_calendar_id,
                destination_calendar_id,
                event,
                access_token,
            )
            .await
        }
        SyncAction::Create | SyncAction::Update => {
            match crate::apis::google_calendar::publish(&event, access_token).await {
                Ok(remote_id) => finalize_event(pool, event, remote_id).await,
                // The event may already live in another calendar (for example a
                // calendar change queued before moves were understood): recover
                // by relocating it to the calendar the payload targets.
                Err(publish_error) => match relocate_event(pool, &event, access_token).await {
                    Some(remote_id) => finalize_event(pool, event, remote_id).await,
                    None => Err(publish_error.to_string()),
                },
            }
        }
    }
}

async fn push_event_move(
    pool: &SqlitePool,
    source_calendar_id: Option<String>,
    destination_calendar_id: Option<String>,
    event: AppEvent,
    access_token: &str,
) -> Result<(), String> {
    let Some(remote_id) = event.remote_id.clone() else {
        return Ok(());
    };

    let moved_id = match source_calendar_id {
        Some(source) => {
            let destination = destination_for(&event, destination_calendar_id.as_deref());
            crate::apis::google_calendar::move_event(
                &remote_id,
                &source,
                &destination,
                access_token,
            )
            .await
            .map_err(|e| e.to_string())?
        }
        None => relocate_event(pool, &event, access_token)
            .await
            .ok_or_else(|| "Could not locate the event in any calendar to move it".to_string())?,
    };

    finalize_event(pool, event, moved_id).await
}

fn destination_for(event: &AppEvent, destination_calendar_id: Option<&str>) -> String {
    destination_calendar_id
        .or_else(|| event.remote_collection_id.as_deref().map(String::as_str))
        .unwrap_or("primary")
        .to_string()
}

async fn relocate_event(
    pool: &SqlitePool,
    event: &AppEvent,
    access_token: &str,
) -> Option<String> {
    let remote_id = event.remote_id.clone()?;
    let destination = destination_for(event, None);
    let calendars = crate::db::get_calendars(pool).await.ok()?;
    let candidates: Vec<String> = calendars
        .into_iter()
        .map(|calendar| calendar.id.0)
        .filter(|id| id != &destination)
        .collect();

    let source =
        crate::apis::google_calendar::locate_event(&remote_id, &candidates, access_token).await?;
    crate::apis::google_calendar::move_event(&remote_id, &source, &destination, access_token)
        .await
        .ok()
}

async fn finalize_event(
    pool: &SqlitePool,
    mut event: AppEvent,
    remote_id: String,
) -> Result<(), String> {
    event.remote_id = Some(RemoteId(remote_id));
    event.dirty = Some(false);
    crate::db::upsert_event(pool, event)
        .await
        .map_err(|e| e.to_string())
}
