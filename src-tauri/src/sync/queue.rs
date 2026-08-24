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
                self.handle_delete_transition(&transition, item, &indices)
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
        item: SyncQueueItem,
        indices: &ExistingIndices,
    ) -> Result<(), sqlx::Error> {
        if transition == "delete->delete" {
            return Ok(());
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
            self.replace_payload(indices.create, existing, &item.item)
                .await?;
        } else if transition == "update->move" {
            self.replace_payload(indices.update, existing, &item.item)
                .await?;
            queue_store::insert(&self.pool, &item).await?;
        } else if transition == "move->move" {
            if let Some(id) = indices.r#move {
                queue_store::update_action_and_payload(&self.pool, id, &item).await?;
            }
        } else if transition == "move+update->move" {
            self.replace_payload(indices.update, existing, &item.item)
                .await?;
            if let Some(id) = indices.r#move {
                queue_store::update_action_and_payload(&self.pool, id, &item).await?;
            }
        } else if transition == "delete->move" {
            eprintln!("Warning: Attempted to move a deleted item. Ignoring.");
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
