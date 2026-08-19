use crate::domain::AppEvent;
use sqlx::SqlitePool;

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_events(pool: &SqlitePool) -> Result<Vec<AppEvent>, sqlx::Error> {
    println!("db::get_events executing query...");
    let events = sqlx::query_as::<_, AppEvent>("SELECT * FROM events")
        .fetch_all(pool)
        .await?;
    println!("db::get_events fetched rows!");
    Ok(events)
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_event(pool: &SqlitePool, id: &str) -> Result<Option<AppEvent>, sqlx::Error> {
    sqlx::query_as::<_, AppEvent>("SELECT * FROM events WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_dirty_events(pool: &SqlitePool) -> Result<Vec<AppEvent>, sqlx::Error> {
    let mut events = get_events(pool).await?;
    events.retain(|e| e.dirty == Some(true));
    Ok(events)
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn create_event(pool: &SqlitePool, event: AppEvent) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO events (
            id, remote_id, remote_collection_id, task_id, title, description, 
            start_time, end_time, event_type, rrule, exdates, recurring_event_id, 
            original_start_time, cancelled, updated_at, deleted, etag, dirty
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(event.id)
    .bind(event.remote_id)
    .bind(event.remote_collection_id)
    .bind(event.task_id)
    .bind(event.title)
    .bind(event.description)
    .bind(event.start_time)
    .bind(event.end_time)
    .bind(event.event_type)
    .bind(event.rrule)
    .bind(event.exdates.map(sqlx::types::Json))
    .bind(event.recurring_event_id)
    .bind(event.original_start_time)
    .bind(event.cancelled)
    .bind(event.updated_at)
    .bind(event.deleted)
    .bind(event.etag)
    .bind(event.dirty)
    .execute(pool)
    .await?;

    Ok(())
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn update_event(pool: &SqlitePool, event: AppEvent) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE events SET 
            remote_id = ?, remote_collection_id = ?, task_id = ?, title = ?, 
            description = ?, start_time = ?, end_time = ?, event_type = ?, 
            rrule = ?, exdates = ?, recurring_event_id = ?, original_start_time = ?, 
            cancelled = ?, updated_at = ?, deleted = ?, etag = ?, dirty = ?
        WHERE id = ?",
    )
    .bind(event.remote_id)
    .bind(event.remote_collection_id)
    .bind(event.task_id)
    .bind(event.title)
    .bind(event.description)
    .bind(event.start_time)
    .bind(event.end_time)
    .bind(event.event_type)
    .bind(event.rrule)
    .bind(event.exdates.map(sqlx::types::Json))
    .bind(event.recurring_event_id)
    .bind(event.original_start_time)
    .bind(event.cancelled)
    .bind(event.updated_at)
    .bind(event.deleted)
    .bind(event.etag)
    .bind(event.dirty)
    .bind(event.id)
    .execute(pool)
    .await?;

    Ok(())
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn upsert_event(pool: &SqlitePool, event: AppEvent) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO events (
            id, remote_id, remote_collection_id, task_id, title, description, 
            start_time, end_time, event_type, rrule, exdates, recurring_event_id, 
            original_start_time, cancelled, updated_at, deleted, etag, dirty
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET 
            remote_id = excluded.remote_id, 
            remote_collection_id = excluded.remote_collection_id, 
            task_id = excluded.task_id, 
            title = excluded.title, 
            description = excluded.description, 
            start_time = excluded.start_time, 
            end_time = excluded.end_time, 
            event_type = excluded.event_type, 
            rrule = excluded.rrule, 
            exdates = excluded.exdates, 
            recurring_event_id = excluded.recurring_event_id, 
            original_start_time = excluded.original_start_time, 
            cancelled = excluded.cancelled, 
            updated_at = excluded.updated_at, 
            deleted = excluded.deleted, 
            etag = excluded.etag,
            dirty = excluded.dirty",
    )
    .bind(event.id)
    .bind(event.remote_id)
    .bind(event.remote_collection_id)
    .bind(event.task_id)
    .bind(event.title)
    .bind(event.description)
    .bind(event.start_time)
    .bind(event.end_time)
    .bind(event.event_type)
    .bind(event.rrule)
    .bind(event.exdates.map(sqlx::types::Json))
    .bind(event.recurring_event_id)
    .bind(event.original_start_time)
    .bind(event.cancelled)
    .bind(event.updated_at)
    .bind(event.deleted)
    .bind(event.etag)
    .bind(event.dirty)
    .execute(pool)
    .await?;

    Ok(())
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn delete_event(pool: &SqlitePool, id: String) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM events WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
