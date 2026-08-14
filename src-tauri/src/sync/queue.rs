use crate::sync::types::{SyncAction, SyncQueueItem, SyncType};
use sqlx::{Row, SqlitePool};
use std::sync::Arc;

pub struct SyncQueue {
    pool: Arc<SqlitePool>,
}

#[derive(Debug)]
struct ExistingIndices {
    create: Option<i64>,
    update: Option<i64>,
    r#move: Option<i64>,
    delete: Option<i64>,
    all_ids: Vec<i64>,
}

impl SyncQueue {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    async fn get_existing(
        &self,
        item_type: &SyncType,
        item_id: &str,
    ) -> Result<(ExistingIndices, Vec<(i64, SyncQueueItem)>), sqlx::Error> {
        let type_str = match item_type {
            SyncType::Task => "task",
            SyncType::Event => "event",
        };

        let rows = sqlx::query(
            "SELECT id, payload, action FROM sync_queue WHERE item_type = ? AND item_id = ? ORDER BY id ASC",
        )
        .bind(type_str)
        .bind(item_id)
        .fetch_all(&*self.pool)
        .await?;

        let mut existing_items = Vec::new();
        let mut indices = ExistingIndices {
            create: None,
            update: None,
            r#move: None,
            delete: None,
            all_ids: Vec::new(),
        };

        for row in rows {
            let id: i64 = row.get("id");
            let payload: String = row.get("payload");

            if let Ok(item) = serde_json::from_str::<SyncQueueItem>(&payload) {
                indices.all_ids.push(id);
                match item.action {
                    SyncAction::Create => indices.create = Some(id),
                    SyncAction::Update => indices.update = Some(id),
                    SyncAction::Move => indices.r#move = Some(id),
                    SyncAction::Delete => indices.delete = Some(id),
                }
                existing_items.push((id, item));
            }
        }

        Ok((indices, existing_items))
    }

    async fn insert_item(&self, item: &SyncQueueItem) -> Result<(), sqlx::Error> {
        let payload = serde_json::to_string(item).unwrap();
        let type_str = match item.r#type {
            SyncType::Task => "task",
            SyncType::Event => "event",
        };
        let action_str = match item.action {
            SyncAction::Create => "create",
            SyncAction::Update => "update",
            SyncAction::Move => "move",
            SyncAction::Delete => "delete",
        };
        let item_id = item.item.id();

        sqlx::query("INSERT INTO sync_queue (item_type, item_id, action, payload) VALUES (?, ?, ?, ?)")
            .bind(type_str)
            .bind(item_id)
            .bind(action_str)
            .bind(payload)
            .execute(&*self.pool)
            .await?;

        Ok(())
    }

    async fn update_item(&self, id: i64, item: &SyncQueueItem) -> Result<(), sqlx::Error> {
        let payload = serde_json::to_string(item).unwrap();
        let action_str = match item.action {
            SyncAction::Create => "create",
            SyncAction::Update => "update",
            SyncAction::Move => "move",
            SyncAction::Delete => "delete",
        };

        sqlx::query("UPDATE sync_queue SET payload = ?, action = ? WHERE id = ?")
            .bind(payload)
            .bind(action_str)
            .bind(id)
            .execute(&*self.pool)
            .await?;

        Ok(())
    }

    async fn remove_items(&self, ids: &[i64]) -> Result<(), sqlx::Error> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> = sqlx::QueryBuilder::new("DELETE FROM sync_queue WHERE id IN (");
        let mut separated = query_builder.separated(", ");
        for id in ids {
            separated.push_bind(id);
        }
        drop(separated);
        query_builder.push(")");
        query_builder.build().execute(&*self.pool).await?;
        Ok(())
    }

    fn determine_state(indices: &ExistingIndices) -> String {
        if indices.create.is_some() {
            return "create".to_string();
        }
        if indices.delete.is_some() {
            return "delete".to_string();
        }
        if indices.update.is_some() && indices.r#move.is_some() {
            return "move+update".to_string();
        }
        if indices.update.is_some() {
            return "update".to_string();
        }
        if indices.r#move.is_some() {
            return "move".to_string();
        }
        "".to_string()
    }

    pub async fn push(&self, item: SyncQueueItem) -> Result<(), sqlx::Error> {
        let item_id = item.item.id();
        let (indices, existing_items) = self.get_existing(&item.r#type, &item_id).await?;

        if indices.all_ids.is_empty() {
            if item.action == SyncAction::Delete && item.remote_id.is_none() {
                return Ok(());
            }
            self.insert_item(&item).await?;
            return Ok(());
        }

        let existing_state = Self::determine_state(&indices);
        let action_str = match item.action {
            SyncAction::Create => "create",
            SyncAction::Update => "update",
            SyncAction::Move => "move",
            SyncAction::Delete => "delete",
        };
        let transition = format!("{}->{}", existing_state, action_str);

        if item.action == SyncAction::Create {
            if transition == "delete->create" {
                self.remove_items(&indices.all_ids).await?;
                self.insert_item(&item).await?;
            } else {
                println!("Warning: Attempted to recreate an item that already exists in the queue.");
            }
        } else if item.action == SyncAction::Update {
            self.handle_update_transition(&transition, item, &indices, &existing_items)
                .await?;
        } else if item.action == SyncAction::Move {
            self.handle_move_transition(&transition, item, &indices, &existing_items)
                .await?;
        } else if item.action == SyncAction::Delete {
            if transition == "delete->delete" {
                return Ok(());
            }
            self.remove_items(&indices.all_ids).await?;
            if transition != "create->delete" && item.remote_id.is_some() {
                self.insert_item(&item).await?;
            }
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
            if let Some(id) = indices.create {
                if let Some((_, q)) = existing.iter().find(|(i, _)| *i == id) {
                    let mut updated_q = q.clone();
                    updated_q.item = item.item.clone();
                    self.update_item(id, &updated_q).await?;
                }
            }
        } else if transition == "update->update" {
            if let Some(id) = indices.update {
                self.update_item(id, &item).await?;
            }
        } else if transition == "move->update" {
            if let Some(id) = indices.r#move {
                if let Some((_, q)) = existing.iter().find(|(i, _)| *i == id) {
                    let mut updated_q = q.clone();
                    updated_q.item = item.item.clone();
                    self.update_item(id, &updated_q).await?;
                }
            }
            self.insert_item(&item).await?;
        } else if transition == "move+update->update" {
            if let Some(id) = indices.r#move {
                if let Some((_, q)) = existing.iter().find(|(i, _)| *i == id) {
                    let mut updated_q = q.clone();
                    updated_q.item = item.item.clone();
                    self.update_item(id, &updated_q).await?;
                }
            }
            if let Some(id) = indices.update {
                self.remove_items(&[id]).await?;
            }
            self.insert_item(&item).await?;
        } else if transition == "delete->update" {
            println!("Warning: Attempted to update a deleted item. Ignoring.");
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
            if let Some(id) = indices.create {
                if let Some((_, q)) = existing.iter().find(|(i, _)| *i == id) {
                    let mut updated_q = q.clone();
                    updated_q.item = item.item.clone();
                    self.update_item(id, &updated_q).await?;
                }
            }
        } else if transition == "update->move" {
            if let Some(id) = indices.update {
                if let Some((_, q)) = existing.iter().find(|(i, _)| *i == id) {
                    let mut updated_q = q.clone();
                    updated_q.item = item.item.clone();
                    self.update_item(id, &updated_q).await?;
                }
            }
            self.insert_item(&item).await?;
        } else if transition == "move->move" {
            if let Some(id) = indices.r#move {
                self.update_item(id, &item).await?;
            }
        } else if transition == "move+update->move" {
            if let Some(id) = indices.update {
                if let Some((_, q)) = existing.iter().find(|(i, _)| *i == id) {
                    let mut updated_q = q.clone();
                    updated_q.item = item.item.clone();
                    self.update_item(id, &updated_q).await?;
                }
            }
            if let Some(id) = indices.r#move {
                self.update_item(id, &item).await?;
            }
        } else if transition == "delete->move" {
            println!("Warning: Attempted to move a deleted item. Ignoring.");
        }
        Ok(())
    }

    pub async fn shift(&self) -> Result<Option<(i64, SyncQueueItem)>, sqlx::Error> {
        let row = sqlx::query("SELECT id, payload FROM sync_queue ORDER BY id ASC LIMIT 1")
            .fetch_optional(&*self.pool)
            .await?;

        if let Some(r) = row {
            let id: i64 = r.get("id");
            let payload: String = r.get("payload");

            if let Ok(item) = serde_json::from_str::<SyncQueueItem>(&payload) {
                self.remove_items(&[id]).await?;
                return Ok(Some((id, item)));
            }
        }
        Ok(None)
    }

    pub async fn peek(&self) -> Result<Option<(i64, SyncQueueItem)>, sqlx::Error> {
        let row = sqlx::query("SELECT id, payload FROM sync_queue ORDER BY id ASC LIMIT 1")
            .fetch_optional(&*self.pool)
            .await?;

        if let Some(r) = row {
            let id: i64 = r.get("id");
            let payload: String = r.get("payload");

            if let Ok(item) = serde_json::from_str::<SyncQueueItem>(&payload) {
                return Ok(Some((id, item)));
            }
        }
        Ok(None)
    }

    pub async fn remove(&self, id: i64) -> Result<(), sqlx::Error> {
        self.remove_items(&[id]).await
    }

    pub async fn get_length(&self) -> Result<i64, sqlx::Error> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM sync_queue")
            .fetch_one(&*self.pool)
            .await?;
        let count: i64 = row.get("count");
        Ok(count)
    }

    pub async fn get_items(&self) -> Result<Vec<SyncQueueItem>, sqlx::Error> {
        let rows = sqlx::query("SELECT payload FROM sync_queue ORDER BY id ASC")
            .fetch_all(&*self.pool)
            .await?;

        let mut items = Vec::new();
        for r in rows {
            let payload: String = r.get("payload");
            if let Ok(item) = serde_json::from_str::<SyncQueueItem>(&payload) {
                items.push(item);
            }
        }
        Ok(items)
    }

    pub async fn clear(&self) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM sync_queue")
            .execute(&*self.pool)
            .await?;
        Ok(())
    }
}
