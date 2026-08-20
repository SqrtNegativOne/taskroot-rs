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

use super::FilterColumnExt;

impl FilterColumnExt for crate::domain::EventFilterColumn {
    fn apply_sql(&self, builder: &mut sqlx::QueryBuilder<sqlx::Sqlite>, op: &str, val: &serde_json::Value) {
        let is_not = op == "is not";
        let values = val.as_array().map_or_else(|| vec![val.clone()], std::clone::Clone::clone);
        if values.is_empty() {
            return;
        }

        match self {
            Self::EventType => {
                builder.push(if is_not { " AND event_type NOT IN (" } else { " AND event_type IN (" });
                let mut separated = builder.separated(", ");
                for v in &values {
                    if let Some(s) = v.as_str() {
                        separated.push_bind(format!("\"{s}\""));
                    } else {
                        separated.push_bind(v.to_string());
                    }
                }
                builder.push(")");
            }
            Self::Calendar => {
                builder.push(if is_not { " AND remote_collection_id NOT IN (" } else { " AND remote_collection_id IN (" });
                let mut separated = builder.separated(", ");
                for v in &values {
                    if let Some(s) = v.as_str() {
                        separated.push_bind(s.to_string());
                    } else {
                        separated.push_bind(v.to_string());
                    }
                }
                builder.push(")");
            }
        }
    }
}

pub async fn get_filtered_events(
    pool: &SqlitePool,
    filters: Vec<crate::domain::AppEventFilter>,
    query_text: String,
) -> Result<Vec<AppEvent>, sqlx::Error> {
    let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> =
        sqlx::QueryBuilder::new("SELECT * FROM events WHERE 1=1");

    for f in &filters {
        if let (Some(col), Some(val)) = (&f.column, &f.value) {
            let op = f.operator.as_deref().unwrap_or("is");
            col.apply_sql(&mut query_builder, op, val);
        }
    }

    if !query_text.trim().is_empty() {
        let q = format!("%{}%", query_text.trim().to_lowercase());
        query_builder.push(" AND (LOWER(title) LIKE ");
        query_builder.push_bind(q.clone());
        query_builder.push(" OR LOWER(description) LIKE ");
        query_builder.push_bind(q);
        query_builder.push(")");
    }

    let events = query_builder.build_query_as::<AppEvent>().fetch_all(pool).await?;

    Ok(events)
}
