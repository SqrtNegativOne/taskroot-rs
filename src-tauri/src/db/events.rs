use crate::domain::AppEvent;
use sqlx::{Row, SqlitePool};

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_events(pool: &SqlitePool) -> Result<Vec<AppEvent>, sqlx::Error> {
    println!("db::get_events executing query...");
    let rows = sqlx::query("SELECT * FROM events").fetch_all(pool).await?;
    println!("db::get_events fetched rows!");
    let mut events = Vec::new();

    for row in rows {
        let type_str: String = row.try_get("event_type")?;
        let exdates_str: Option<String> = row.try_get("exdates")?;

        events.push(AppEvent {
            id: row.try_get("id")?,
            remote_id: row.try_get("remote_id")?,
            remote_collection_id: row.try_get("remote_collection_id")?,
            task_id: row.try_get("task_id")?,
            title: row.try_get("title")?,
            description: row.try_get("description")?,
            start_time: row.try_get("start_time")?,
            end_time: row.try_get("end_time")?,
            event_type: serde_json::from_value(serde_json::Value::String(type_str))
                .unwrap_or(crate::domain::AppEventType::Info),
            rrule: row.try_get("rrule")?,
            exdates: exdates_str.and_then(|s| serde_json::from_str(&s).ok()),
            recurring_event_id: row.try_get("recurring_event_id")?,
            original_start_time: row.try_get("original_start_time")?,
            cancelled: row.try_get("cancelled")?,
            updated_at: row.try_get("updated_at")?,
            color: None, // We don't have it in db yet
            deleted: row.try_get("deleted")?,
            etag: row.try_get("etag")?,
            dirty: row.try_get("dirty").unwrap_or(Some(false)),
        });
    }

    Ok(events)
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_event(pool: &SqlitePool, id: &str) -> Result<Option<AppEvent>, sqlx::Error> {
    let row = sqlx::query("SELECT * FROM events WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    if let Some(r) = row {
        let type_str: String = r.try_get("event_type")?;
        let exdates_str: Option<String> = r.try_get("exdates")?;

        Ok(Some(AppEvent {
            id: r.try_get("id")?,
            remote_id: r.try_get("remote_id")?,
            remote_collection_id: r.try_get("remote_collection_id")?,
            task_id: r.try_get("task_id")?,
            title: r.try_get("title")?,
            description: r.try_get("description")?,
            start_time: r.try_get("start_time")?,
            end_time: r.try_get("end_time")?,
            event_type: serde_json::from_value(serde_json::Value::String(type_str))
                .unwrap_or(crate::domain::AppEventType::Info),
            rrule: r.try_get("rrule")?,
            exdates: exdates_str.and_then(|s| serde_json::from_str(&s).ok()),
            recurring_event_id: r.try_get("recurring_event_id")?,
            original_start_time: r.try_get("original_start_time")?,
            cancelled: r.try_get("cancelled")?,
            updated_at: r.try_get("updated_at")?,
            color: None,
            deleted: r.try_get("deleted")?,
            etag: r.try_get("etag")?,
            dirty: r.try_get("dirty").unwrap_or(Some(false)),
        }))
    } else {
        Ok(None)
    }
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
    let type_str = serde_json::to_string(&event.event_type).unwrap_or_default();
    let exdates_str = event
        .exdates
        .map(|e| serde_json::to_string(&e).unwrap_or_default());

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
    .bind(type_str.trim_matches('"'))
    .bind(event.rrule)
    .bind(exdates_str)
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
    let type_str = serde_json::to_string(&event.event_type).unwrap_or_default();
    let exdates_str = event
        .exdates
        .map(|e| serde_json::to_string(&e).unwrap_or_default());

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
    .bind(type_str.trim_matches('"'))
    .bind(event.rrule)
    .bind(exdates_str)
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
    let type_str = serde_json::to_string(&event.event_type).unwrap_or_default();
    let exdates_str = event
        .exdates
        .map(|e| serde_json::to_string(&e).unwrap_or_default());

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
    .bind(type_str.trim_matches('"'))
    .bind(event.rrule)
    .bind(exdates_str)
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
