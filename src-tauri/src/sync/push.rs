use crate::auth;
use crate::domain::{AppEvent, AppTask};
use crate::sync::queue::SyncQueue;
use crate::sync::types::{SyncAction, SyncItemData, SyncQueueItem, SyncType};
use sqlx::SqlitePool;
use std::future::Future;
use std::sync::Arc;

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
    ) -> impl Future<Output = anyhow::Result<String>> + Send;

    fn delete_remote(
        &self,
        remote_id: &str,
        access_token: &str,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

impl GoogleSyncEntity for AppTask {
    const SYNC_TYPE: SyncType = SyncType::Task;

    fn remote_id(&self) -> Option<&String> {
        self.remote_id.as_ref()
    }

    fn to_item_data(&self) -> SyncItemData {
        SyncItemData::Task(self.clone())
    }

    fn mark_updated(&mut self) {
        self.dirty = Some(true);
        self.updated_at = Some(chrono::Utc::now().timestamp_millis());
    }

    fn mark_clean(&mut self) {
        self.dirty = Some(false);
    }

    fn set_remote_id(&mut self, remote_id: String) {
        self.remote_id = Some(remote_id);
    }

    async fn publish_remote(&self, access_token: &str) -> anyhow::Result<String> {
        crate::apis::google_tasks::publish(self, access_token).await
    }

    async fn delete_remote(&self, remote_id: &str, access_token: &str) -> anyhow::Result<()> {
        crate::apis::google_tasks::delete(remote_id, access_token).await
    }
}

impl GoogleSyncEntity for AppEvent {
    const SYNC_TYPE: SyncType = SyncType::Event;

    fn remote_id(&self) -> Option<&String> {
        self.remote_id.as_ref()
    }

    fn to_item_data(&self) -> SyncItemData {
        SyncItemData::Event(self.clone())
    }

    fn mark_updated(&mut self) {
        self.dirty = Some(true);
        self.updated_at = Some(chrono::Utc::now().timestamp_millis());
    }

    fn mark_clean(&mut self) {
        self.dirty = Some(false);
    }

    fn set_remote_id(&mut self, remote_id: String) {
        self.remote_id = Some(remote_id);
    }

    async fn publish_remote(&self, access_token: &str) -> anyhow::Result<String> {
        crate::apis::google_calendar::publish(self, access_token).await
    }

    async fn delete_remote(&self, remote_id: &str, access_token: &str) -> anyhow::Result<()> {
        crate::apis::google_calendar::delete(remote_id, access_token).await
    }
}

fn queue_item<T: GoogleSyncEntity>(
    entity: &T,
    action: SyncAction,
    remote_id: Option<String>,
) -> SyncQueueItem {
    SyncQueueItem {
        r#type: T::SYNC_TYPE,
        action,
        item: entity.to_item_data(),
        remote_id,
        calendar_id: None,
        destination_calendar_id: None,
        updated_fields: None,
    }
}

async fn enqueue(item: SyncQueueItem, pool: &SqlitePool) {
    let queue = SyncQueue::new(Arc::new(pool.clone()));
    let _ = queue.push(item).await;
}

/// Publishes the entity to Google, falling back to enqueueing for offline retry.
/// On success the entity gains its remote id and is marked clean.
pub async fn push_or_enqueue<T: GoogleSyncEntity>(
    pool: &SqlitePool,
    entity: &mut T,
    action: SyncAction,
) {
    entity.mark_updated();

    if let Ok(access_token) = auth::get_valid_access_token(pool).await {
        if let Ok(remote_id) = entity.publish_remote(&access_token).await {
            entity.set_remote_id(remote_id);
            entity.mark_clean();
            return;
        }
    }

    enqueue(queue_item(entity, action, None), pool).await;
}

/// Deletes the entity remotely when it has a remote id, otherwise treats it as
/// done locally. Falls back to enqueueing the delete for offline retry.
pub async fn push_delete_or_enqueue<T: GoogleSyncEntity>(pool: &SqlitePool, entity: &T) {
    let deleted_remotely = match entity.remote_id() {
        Some(remote_id) => {
            matches!(
                auth::get_valid_access_token(pool).await,
                Ok(access_token) if entity.delete_remote(remote_id, &access_token).await.is_ok()
            )
        }
        None => true,
    };

    if !deleted_remotely {
        let remote_id = entity.remote_id().cloned();
        enqueue(queue_item(entity, SyncAction::Delete, remote_id), pool).await;
    }
}
