use super::queue_store;
use crate::sync::types::{SyncAction, SyncItemData, SyncQueueItem, SyncType};
use sqlx::SqlitePool;
use std::sync::Arc;

pub struct SyncQueue {
    pool: Arc<SqlitePool>,
}

#[derive(Debug, Default)]
struct ExistingIndices {
    create: Option<i64>,
    update: Option<i64>,
    r#move: Option<i64>,
    delete: Option<i64>,
    all_ids: Vec<i64>,
}

impl ExistingIndices {
    fn record(&mut self, id: i64, action: &SyncAction) {
        self.all_ids.push(id);
        match action {
            SyncAction::Create => self.create = Some(id),
            SyncAction::Update => self.update = Some(id),
            SyncAction::Move => self.r#move = Some(id),
            SyncAction::Delete => self.delete = Some(id),
        }
    }

    fn state(&self) -> String {
        if self.create.is_some() {
            "create"
        } else if self.delete.is_some() {
            "delete"
        } else if self.update.is_some() && self.r#move.is_some() {
            "move+update"
        } else if self.update.is_some() {
            "update"
        } else if self.r#move.is_some() {
            "move"
        } else {
            ""
        }
        .to_string()
    }
}

impl SyncQueue {
    #[must_use]
    pub const fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    async fn load_existing(
        &self,
        item_type: &SyncType,
        item_id: &str,
    ) -> Result<(ExistingIndices, Vec<(i64, SyncQueueItem)>), sqlx::Error> {
        let rows = queue_store::fetch_by_item(&self.pool, &item_type.to_string(), item_id).await?;

        let mut indices = ExistingIndices::default();
        let mut existing_items = Vec::new();
        for row in rows {
            indices.record(row.id, &row.item.action);
            existing_items.push((row.id, row.item));
        }
        Ok((indices, existing_items))
    }

    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub async fn push(&self, item: SyncQueueItem) -> Result<(), sqlx::Error> {
        let item_id = item.item.id();
        let (indices, existing_items) = self.load_existing(&item.r#type, &item_id).await?;

        if indices.all_ids.is_empty() {
            if item.action == SyncAction::Delete && item.remote_id.is_none() {
                return Ok(());
            }
            return queue_store::insert(&self.pool, &item).await;
        }

        let transition = format!("{}->{}", indices.state(), item.action);

        match item.action {
            SyncAction::Create => {
                self.handle_create_transition(&transition, item, &indices)
                    .await?;
            }
            SyncAction::Update => {
                self.handle_update_transition(&transition, item, &indices, &existing_items)
                    .await?;
            }
            SyncAction::Move => {
                self.handle_move_transition(&transition, item, &indices, &existing_items)
                    .await?;
            }
            SyncAction::Delete => {
                self.handle_delete_transition(&transition, item, &indices, &existing_items)
                    .await?;
            }
        }

        Ok(())
    }

    async fn handle_create_transition(
        &self,
        transition: &str,
        item: SyncQueueItem,
        indices: &ExistingIndices,
    ) -> Result<(), sqlx::Error> {
        if transition == "delete->create" {
            queue_store::remove_ids(&self.pool, &indices.all_ids).await?;
            queue_store::insert(&self.pool, &item).await?;
        } else {
            eprintln!("Warning: Attempted to recreate an item that already exists in the queue.");
        }
        Ok(())
    }

    async fn handle_delete_transition(
        &self,
        transition: &str,
        mut item: SyncQueueItem,
        indices: &ExistingIndices,
        existing: &[(i64, SyncQueueItem)],
    ) -> Result<(), sqlx::Error> {
        if transition == "delete->delete" {
            return Ok(());
        }
        // A pending move has not reached Google yet, so the delete must still be
        // issued against the calendar the event currently lives in.
        if let Some((_, move_item)) = indices
            .r#move
            .and_then(|id| existing.iter().find(|(existing_id, _)| *existing_id == id))
        {
            if item.calendar_id.is_none() {
                item.calendar_id = move_item.calendar_id.clone();
            }
        }
        queue_store::remove_ids(&self.pool, &indices.all_ids).await?;
        if transition != "create->delete" && item.remote_id.is_some() {
            queue_store::insert(&self.pool, &item).await?;
        }
        Ok(())
    }

    async fn handle_update_transition(
        &self,
        transition: &str,
        item: SyncQueueItem,
        indices: &ExistingIndices,
        existing: &[(i64, SyncQueueItem)],
    ) -> Result<(), sqlx::Error> {
        if transition == "create->update" {
            self.replace_payload(indices.create, existing, &item.item)
                .await?;
        } else if transition == "update->update" {
            if let Some(id) = indices.update {
                queue_store::update_action_and_payload(&self.pool, id, &item).await?;
            }
        } else if transition == "move->update" {
            self.replace_payload(indices.r#move, existing, &item.item)
                .await?;
            queue_store::insert(&self.pool, &item).await?;
        } else if transition == "move+update->update" {
            self.replace_payload(indices.r#move, existing, &item.item)
                .await?;
            if let Some(id) = indices.update {
                queue_store::remove_ids(&self.pool, &[id]).await?;
            }
            queue_store::insert(&self.pool, &item).await?;
        } else if transition == "delete->update" {
            eprintln!("Warning: Attempted to update a deleted item. Ignoring.");
        }
        Ok(())
    }

    async fn handle_move_transition(
        &self,
        transition: &str,
        item: SyncQueueItem,
        indices: &ExistingIndices,
        existing: &[(i64, SyncQueueItem)],
    ) -> Result<(), sqlx::Error> {
        if transition == "create->move" {
            // Still local-only: the create simply targets the new calendar.
            self.replace_payload(indices.create, existing, &item.item)
                .await?;
        } else if transition == "update->move" {
            // The move carries the full payload, so it subsumes the update.
            self.convert_to_move(indices.update, &item).await?;
        } else if transition == "move->move" {
            self.convert_to_move(indices.r#move, &item).await?;
        } else if transition == "move+update->move" {
            if let Some(id) = indices.update {
                queue_store::remove_ids(&self.pool, &[id]).await?;
            }
            self.convert_to_move(indices.r#move, &item).await?;
        } else if transition == "delete->move" {
            eprintln!("Warning: Attempted to move a deleted item. Ignoring.");
        }
        Ok(())
    }

    async fn convert_to_move(
        &self,
        target_id: Option<i64>,
        item: &SyncQueueItem,
    ) -> Result<(), sqlx::Error> {
        if let Some(id) = target_id {
            queue_store::update_action_and_payload(&self.pool, id, item).await?;
        }
        Ok(())
    }

    async fn replace_payload(
        &self,
        target_id: Option<i64>,
        existing: &[(i64, SyncQueueItem)],
        new_item: &SyncItemData,
    ) -> Result<(), sqlx::Error> {
        if let Some((id, q)) = target_id.and_then(|id| existing.iter().find(|(i, _)| *i == id)) {
            let mut updated_q = q.clone();
            updated_q.item = new_item.clone();
            queue_store::update_action_and_payload(&self.pool, *id, &updated_q).await?;
        }
        Ok(())
    }

    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub async fn peek(&self) -> Result<Option<(i64, SyncQueueItem)>, sqlx::Error> {
        Ok(queue_store::fetch_oldest(&self.pool)
            .await?
            .map(|row| (row.id, row.item)))
    }

    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub async fn remove(&self, id: i64) -> Result<(), sqlx::Error> {
        queue_store::remove_ids(&self.pool, &[id]).await
    }

    /// Pending-item count for the offline-sync roadmap (queue badge/telemetry).
    #[allow(dead_code)]
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub async fn get_length(&self) -> Result<i64, sqlx::Error> {
        queue_store::count(&self.pool).await
    }

    /// Full pending list for the offline-sync roadmap (queue inspection).
    #[allow(dead_code)]
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub async fn get_items(&self) -> Result<Vec<SyncQueueItem>, sqlx::Error> {
        Ok(queue_store::fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| row.item)
            .collect())
    }

    /// Queue reset for the offline-sync roadmap (e.g. sign-out / clear-all-data).
    #[allow(dead_code)]
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub async fn clear(&self) -> Result<(), sqlx::Error> {
        queue_store::clear(&self.pool).await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::domain::{AppEvent, CollectionId, EventId, RemoteId};

    fn event(calendar: &str, remote_id: Option<&str>) -> SyncItemData {
        SyncItemData::Event(AppEvent {
            id: EventId("google-r1".into()),
            remote_id: remote_id.map(|id| RemoteId(id.into())),
            remote_collection_id: Some(CollectionId(calendar.into())),
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
        })
    }

    fn item(
        action: SyncAction,
        calendar: &str,
        remote_id: Option<&str>,
        source: Option<&str>,
        destination: Option<&str>,
    ) -> SyncQueueItem {
        SyncQueueItem {
            r#type: SyncType::Event,
            action,
            item: event(calendar, remote_id),
            remote_id: remote_id.map(str::to_owned),
            calendar_id: source.map(str::to_owned),
            destination_calendar_id: destination.map(str::to_owned),
            updated_fields: None,
        }
    }

    async fn queue() -> SyncQueue {
        let pool = crate::db::init_db("sqlite::memory:").await.unwrap();
        SyncQueue::new(Arc::new(pool))
    }

    async fn stored(queue: &SyncQueue) -> Vec<SyncQueueItem> {
        queue_store::fetch_all(&queue.pool)
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.item)
            .collect()
    }

    #[tokio::test]
    async fn update_then_move_collapses_into_one_move() {
        let queue = queue().await;
        queue
            .push(item(SyncAction::Update, "routine", Some("r1"), None, None))
            .await
            .unwrap();
        queue
            .push(item(
                SyncAction::Move,
                "routine",
                Some("r1"),
                Some("busy"),
                Some("routine"),
            ))
            .await
            .unwrap();

        let rows = stored(&queue).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, SyncAction::Move);
        assert_eq!(rows[0].calendar_id.as_deref(), Some("busy"));
        assert_eq!(rows[0].destination_calendar_id.as_deref(), Some("routine"));
    }

    #[tokio::test]
    async fn create_then_move_only_retargets_the_create() {
        let queue = queue().await;
        queue
            .push(item(SyncAction::Create, "busy", None, None, None))
            .await
            .unwrap();
        queue
            .push(item(
                SyncAction::Move,
                "routine",
                None,
                Some("busy"),
                Some("routine"),
            ))
            .await
            .unwrap();

        let rows = stored(&queue).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, SyncAction::Create);
        let SyncItemData::Event(event) = &rows[0].item else {
            panic!("expected an event payload");
        };
        assert_eq!(
            event.remote_collection_id.as_ref().map(|id| id.0.as_str()),
            Some("routine")
        );
    }

    #[tokio::test]
    async fn move_then_move_keeps_one_row_targeting_the_latest_calendar() {
        let queue = queue().await;
        queue
            .push(item(
                SyncAction::Move,
                "routine",
                Some("r1"),
                Some("busy"),
                Some("routine"),
            ))
            .await
            .unwrap();
        queue
            .push(item(
                SyncAction::Move,
                "major",
                Some("r1"),
                Some("busy"),
                Some("major"),
            ))
            .await
            .unwrap();

        let rows = stored(&queue).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, SyncAction::Move);
        assert_eq!(rows[0].destination_calendar_id.as_deref(), Some("major"));
    }

    #[tokio::test]
    async fn delete_after_a_pending_move_targets_the_source_calendar() {
        let queue = queue().await;
        queue
            .push(item(
                SyncAction::Move,
                "routine",
                Some("r1"),
                Some("busy"),
                Some("routine"),
            ))
            .await
            .unwrap();
        queue
            .push(item(SyncAction::Delete, "routine", Some("r1"), None, None))
            .await
            .unwrap();

        let rows = stored(&queue).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, SyncAction::Delete);
        assert_eq!(rows[0].calendar_id.as_deref(), Some("busy"));
    }
}
