use crate::sync::types::SyncQueueItem;
use sqlx::SqlitePool;

pub(super) struct QueueRow {
    pub id: i64,
    pub item: SyncQueueItem,
}

fn decode_row(id: i64, payload: &str) -> Option<QueueRow> {
    serde_json::from_str::<SyncQueueItem>(payload)
        .ok()
        .map(|item| QueueRow { id, item })
}

pub(super) async fn fetch_by_item(
    pool: &SqlitePool,
    item_type: &str,
    item_id: &str,
) -> Result<Vec<QueueRow>, sqlx::Error> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, payload FROM sync_queue WHERE item_type = ? AND item_id = ? ORDER BY id ASC",
    )
    .bind(item_type)
    .bind(item_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|(id, payload)| decode_row(id, &payload))
        .collect())
}

pub(super) async fn fetch_oldest(pool: &SqlitePool) -> Result<Option<QueueRow>, sqlx::Error> {
    let row: Option<(i64, String)> =
        sqlx::query_as("SELECT id, payload FROM sync_queue ORDER BY id ASC LIMIT 1")
            .fetch_optional(pool)
            .await?;
    Ok(row.and_then(|(id, payload)| decode_row(id, &payload)))
}

pub(super) async fn fetch_all(pool: &SqlitePool) -> Result<Vec<QueueRow>, sqlx::Error> {
    let rows: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, payload FROM sync_queue ORDER BY id ASC")
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(id, payload)| decode_row(id, &payload))
        .collect())
}

pub(super) async fn insert(pool: &SqlitePool, item: &SyncQueueItem) -> Result<(), sqlx::Error> {
    let payload = serde_json::to_string(item).map_err(|e| sqlx::Error::Protocol(e.to_string()))?;

    sqlx::query("INSERT INTO sync_queue (item_type, item_id, action, payload) VALUES (?, ?, ?, ?)")
        .bind(item.r#type.to_string())
        .bind(item.item.id())
        .bind(item.action.to_string())
        .bind(payload)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn update_action_and_payload(
    pool: &SqlitePool,
    id: i64,
    item: &SyncQueueItem,
) -> Result<(), sqlx::Error> {
    let payload = serde_json::to_string(item).map_err(|e| sqlx::Error::Protocol(e.to_string()))?;

    sqlx::query("UPDATE sync_queue SET payload = ?, action = ? WHERE id = ?")
        .bind(payload)
        .bind(item.action.to_string())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn remove_ids(pool: &SqlitePool, ids: &[i64]) -> Result<(), sqlx::Error> {
    if ids.is_empty() {
        return Ok(());
    }

    let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> =
        sqlx::QueryBuilder::new("DELETE FROM sync_queue WHERE id IN (");
    let mut separated = query_builder.separated(", ");
    for id in ids {
        separated.push_bind(id);
    }
    query_builder.push(")");
    query_builder.build().execute(pool).await?;
    Ok(())
}

pub(super) async fn count(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) as count FROM sync_queue")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

pub(super) async fn clear(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sync_queue").execute(pool).await?;
    Ok(())
}
