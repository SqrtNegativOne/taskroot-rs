use crate::domain::{AppEvent, AppTask};
use crate::sync::queue::SyncQueue;
use crate::sync::types::{SyncAction, SyncItemData, SyncQueueItem, SyncType};
use sqlx::SqlitePool;
use std::future::Future;
use std::sync::Arc;

/// How a locally edited event should reach Google.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventSyncPlan {
    /// Same calendar: a plain update is enough.
    Update,
    /// The event changed calendars, so it must be moved from the source.
    Move { source_calendar_id: String },
}

const PRIMARY_CALENDAR: &str = "primary";

/// Decides whether editing `incoming` relocated an existing remote event.
///
/// A move can only be remote when the row already has a `remote_id`; a local-only
/// event just needs the create/update to target the new calendar.
#[must_use]
pub fn plan_event_sync(previous: Option<&AppEvent>, incoming: &AppEvent) -> EventSyncPlan {
    let Some(previous) = previous else {
        return EventSyncPlan::Update;
    };
    if incoming.remote_id.is_none() {
        return EventSyncPlan::Update;
    }

    let source = calendar_or_primary(previous.remote_collection_id.as_deref().map(String::as_str));
    let destination =
        calendar_or_primary(incoming.remote_collection_id.as_deref().map(String::as_str));
    if source == destination {
        return EventSyncPlan::Update;
    }
    EventSyncPlan::Move {
        source_calendar_id: source,
    }
}

fn calendar_or_primary(calendar_id: Option<&str>) -> String {
    calendar_id.unwrap_or(PRIMARY_CALENDAR).to_string()
}

pub trait GoogleSyncEntity: Clone + Send + Sync {
    const SYNC_TYPE: SyncType;

    fn remote_id(&self) -> Option<&String>;
    fn to_item_data(&self) -> SyncItemData;
    fn mark_updated(&mut self);
    fn mark_clean(&mut self);
    fn set_remote_id(&mut self, remote_id: String);

    fn publish_remote(
        &self,
        access_token: &str,
    ) -> impl Future<Output = color_eyre::Result<String>> + Send;

    fn delete_remote(
        &self,
        remote_id: &str,
        access_token: &str,
    ) -> impl Future<Output = color_eyre::Result<()>> + Send;
}

impl GoogleSyncEntity for AppTask {
    const SYNC_TYPE: SyncType = SyncType::Task;

    fn remote_id(&self) -> Option<&String> {
        self.remote_id.as_deref()
    }

    fn to_item_data(&self) -> SyncItemData {
        SyncItemData::Task(self.clone())
    }

    fn mark_updated(&mut self) {
        self.dirty = Some(true);
        self.updated_at = Some(chrono::Utc::now().to_rfc3339());
    }

    fn mark_clean(&mut self) {
        self.dirty = Some(false);
    }

    fn set_remote_id(&mut self, remote_id: String) {
        self.remote_id = Some(crate::domain::RemoteId(remote_id));
    }

    async fn publish_remote(&self, access_token: &str) -> color_eyre::Result<String> {
        crate::apis::google_tasks::publish(self, access_token).await
    }

    async fn delete_remote(&self, remote_id: &str, access_token: &str) -> color_eyre::Result<()> {
        crate::apis::google_tasks::delete(remote_id, access_token).await
    }
}

impl GoogleSyncEntity for AppEvent {
    const SYNC_TYPE: SyncType = SyncType::Event;

    fn remote_id(&self) -> Option<&String> {
        self.remote_id.as_deref()
    }

    fn to_item_data(&self) -> SyncItemData {
        SyncItemData::Event(self.clone())
    }

    fn mark_updated(&mut self) {
        self.dirty = Some(true);
        self.updated_at = Some(chrono::Utc::now().to_rfc3339());
    }

    fn mark_clean(&mut self) {
        self.dirty = Some(false);
    }

    fn set_remote_id(&mut self, remote_id: String) {
        self.remote_id = Some(crate::domain::RemoteId(remote_id));
    }

    async fn publish_remote(&self, access_token: &str) -> color_eyre::Result<String> {
        crate::apis::google_calendar::publish(self, access_token).await
    }

    async fn delete_remote(&self, remote_id: &str, access_token: &str) -> color_eyre::Result<()> {
        crate::apis::google_calendar::delete(
            remote_id,
            self.remote_collection_id
                .as_deref()
                .map(std::string::String::as_str),
            access_token,
        )
        .await
    }
}

fn queue_item<T: GoogleSyncEntity>(
    entity: &T,
    action: SyncAction,
    remote_id: Option<String>,
    calendars: (Option<String>, Option<String>),
) -> SyncQueueItem {
    SyncQueueItem {
        r#type: T::SYNC_TYPE,
        action,
        item: entity.to_item_data(),
        remote_id,
        calendar_id: calendars.0,
        destination_calendar_id: calendars.1,
        updated_fields: None,
    }
}

async fn enqueue(item: SyncQueueItem, pool: &SqlitePool) {
    let queue = SyncQueue::new(Arc::new(pool.clone()));
    let _ = queue.push(item).await;
}

pub async fn push_or_enqueue<T: GoogleSyncEntity>(
    app: &tauri::AppHandle,
    entity: &mut T,
    action: SyncAction,
) {
    if let Ok(pool) = crate::db_pool(app) {
        enqueue_update(&pool, entity, action).await;
        crate::sync::trigger_sync(app);
    }
}

/// Marks `entity` dirty and queues `action` for the next sync.
///
/// Split out of the `AppHandle` wrapper so the offline-queue behavior can be
/// unit tested against an in-memory pool.
async fn enqueue_update<T: GoogleSyncEntity>(
    pool: &SqlitePool,
    entity: &mut T,
    action: SyncAction,
) {
    entity.mark_updated();
    let remote_id = entity.remote_id().cloned();
    enqueue(queue_item(entity, action, remote_id, (None, None)), pool).await;
}

/// Enqueues a calendar relocation so the push path can call Google's
/// `events.move` with the calendar the event currently lives in.
pub async fn push_event_move_or_enqueue(
    app: &tauri::AppHandle,
    event: &mut AppEvent,
    source_calendar_id: String,
) {
    if let Ok(pool) = crate::db_pool(app) {
        event.mark_updated();
        let remote_id = event.remote_id().cloned();
        let destination = event.remote_collection_id.as_deref().cloned();
        enqueue(
            queue_item(
                event,
                SyncAction::Move,
                remote_id,
                (Some(source_calendar_id), destination),
            ),
            &pool,
        )
        .await;
        crate::sync::trigger_sync(app);
    }
}

/// Enqueues a delete action for offline retry.
pub async fn push_delete_or_enqueue<T: GoogleSyncEntity>(app: &tauri::AppHandle, entity: &T) {
    if let Ok(pool) = crate::db_pool(app) {
        let remote_id = entity.remote_id().cloned();
        enqueue(queue_item(entity, SyncAction::Delete, remote_id, (None, None)), &pool).await;
        crate::sync::trigger_sync(app);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::domain::{CollectionId, EventId, RemoteId};

    fn event(calendar: Option<&str>, remote_id: Option<&str>) -> AppEvent {
        AppEvent {
            id: EventId("local-1".into()),
            remote_id: remote_id.map(|id| RemoteId(id.into())),
            remote_collection_id: calendar.map(|id| CollectionId(id.into())),
            task_id: None,
            title: "Drink water".into(),
            description: None,
            start_time: "2026-06-14T19:00:00+05:30".into(),
            end_time: "2026-06-14T19:05:00+05:30".into(),
            rrule: None,
            exdates: None,
            recurring_event_id: None,
            original_start_time: None,
            status: None,
            updated_at: None,
            color: None,
            etag: None,
            dirty: Some(true),
            is_all_day: Some(false),
            timezone: None,
        }
    }

    #[test]
    fn unchanged_calendar_is_a_plain_update() {
        let previous = event(Some("busy"), Some("remote-1"));
        let incoming = event(Some("busy"), Some("remote-1"));
        assert_eq!(plan_event_sync(Some(&previous), &incoming), EventSyncPlan::Update);
    }

    #[test]
    fn calendar_change_on_a_synced_event_is_a_move_from_the_old_calendar() {
        let previous = event(Some("busy"), Some("remote-1"));
        let incoming = event(Some("routine"), Some("remote-1"));
        assert_eq!(
            plan_event_sync(Some(&previous), &incoming),
            EventSyncPlan::Move {
                source_calendar_id: "busy".into()
            }
        );
    }

    #[test]
    fn calendar_change_on_a_local_only_event_stays_an_update() {
        let previous = event(Some("busy"), None);
        let incoming = event(Some("routine"), None);
        assert_eq!(plan_event_sync(Some(&previous), &incoming), EventSyncPlan::Update);
    }

    #[test]
    fn a_new_event_is_never_a_move() {
        let incoming = event(Some("routine"), Some("remote-1"));
        assert_eq!(plan_event_sync(None, &incoming), EventSyncPlan::Update);
    }

    #[tokio::test]
    async fn enqueue_update_marks_dirty_and_queues_the_payload() {
        let pool = crate::db::init_db("sqlite::memory:").await.unwrap();
        let mut pending = event(Some("busy"), Some("remote-1"));
        pending.dirty = Some(false);
        pending.updated_at = None;

        enqueue_update(&pool, &mut pending, SyncAction::Update).await;

        assert_eq!(pending.dirty, Some(true), "queued rows must be marked dirty");
        assert!(pending.updated_at.is_some(), "the edit timestamp must advance");

        let queue = SyncQueue::new(Arc::new(pool));
        let (_, item) = queue.peek().await.unwrap().expect("an update is queued");
        assert_eq!(item.action, SyncAction::Update);
        assert_eq!(item.remote_id.as_deref(), Some("remote-1"));
    }

    #[test]
    fn leaving_the_primary_calendar_moves_from_primary() {
        let previous = event(None, Some("remote-1"));
        let incoming = event(Some("routine"), Some("remote-1"));
        assert_eq!(
            plan_event_sync(Some(&previous), &incoming),
            EventSyncPlan::Move {
                source_calendar_id: "primary".into()
            }
        );
    }
}
