use crate::domain::{AppEvent, AppTask};
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};
use std::str::FromStr;

pub async fn init_db(db_path: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(db_path)?.create_if_missing(true);

    let pool = SqlitePool::connect_with(options).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            status TEXT,
            priority INTEGER,
            tags TEXT,
            subtasks TEXT,
            parent_task TEXT,
            dependencies TEXT,
            est INTEGER,
            added TEXT,
            canvas_x REAL,
            canvas_y REAL,
            on_canvas BOOLEAN,
            remote_id TEXT,
            notes TEXT,
            tabs TEXT,
            due TEXT,
            deleted BOOLEAN,
            updated_at INTEGER,
            etag TEXT,
            dirty BOOLEAN DEFAULT 0
        );",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            remote_id TEXT,
            remote_collection_id TEXT,
            task_id TEXT,
            title TEXT NOT NULL,
            description TEXT,
            start_time TEXT NOT NULL,
            end_time TEXT NOT NULL,
            event_type TEXT NOT NULL,
            rrule TEXT,
            exdates TEXT,
            recurring_event_id TEXT,
            original_start_time TEXT,
            cancelled BOOLEAN,
            updated_at INTEGER,
            deleted BOOLEAN,
            etag TEXT,
            dirty BOOLEAN DEFAULT 0
        );",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sync_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            item_type TEXT NOT NULL,
            item_id TEXT NOT NULL,
            action TEXT NOT NULL,
            payload TEXT NOT NULL
        );",
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

pub async fn get_setting(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.0))
}

pub async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

fn row_to_task(row: &sqlx::sqlite::SqliteRow) -> Result<AppTask, sqlx::Error> {
    let status_str: Option<String> = row.try_get("status")?;
    let tags_str: Option<String> = row.try_get("tags")?;
    let subtasks_str: Option<String> = row.try_get("subtasks")?;
    let dependencies_str: Option<String> = row.try_get("dependencies")?;

    Ok(AppTask {
        id: row.try_get("id")?,
        title: row.try_get("title")?,
        status: status_str
            .and_then(|s| serde_json::from_value(serde_json::Value::String(s)).ok()),
        priority: row.try_get("priority")?,
        tags: tags_str.and_then(|s| serde_json::from_str(&s).ok()),
        subtasks: subtasks_str.and_then(|s| serde_json::from_str(&s).ok()),
        parent_task: row.try_get("parent_task")?,
        dependencies: dependencies_str.and_then(|s| serde_json::from_str(&s).ok()),
        est: row.try_get("est")?,
        added: row.try_get("added")?,
        canvas_x: row.try_get("canvas_x")?,
        canvas_y: row.try_get("canvas_y")?,
        on_canvas: row.try_get("on_canvas")?,
        remote_id: row.try_get("remote_id")?,
        notes: row.try_get("notes")?,
        tabs: row.try_get("tabs")?,
        due: row.try_get("due")?,
        deleted: row.try_get("deleted")?,
        updated_at: row.try_get("updated_at")?,
        etag: row.try_get("etag")?,
        dirty: row.try_get("dirty").unwrap_or(Some(false)),
    })
}

pub async fn get_tasks(pool: &SqlitePool) -> Result<Vec<AppTask>, sqlx::Error> {
    let rows = sqlx::query("SELECT * FROM tasks").fetch_all(pool).await?;
    let mut tasks = Vec::new();

    for row in rows {
        tasks.push(row_to_task(&row)?);
    }

    Ok(tasks)
}

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

pub async fn get_task(pool: &SqlitePool, id: &str) -> Result<Option<AppTask>, sqlx::Error> {
    let row = sqlx::query("SELECT * FROM tasks WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    
    if let Some(r) = row {
        Ok(Some(row_to_task(&r)?))
    } else {
        Ok(None)
    }
}

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

pub async fn get_dirty_tasks(pool: &SqlitePool) -> Result<Vec<AppTask>, sqlx::Error> {
    let mut tasks = get_tasks(pool).await?;
    tasks.retain(|t| t.dirty == Some(true));
    Ok(tasks)
}

pub async fn get_dirty_events(pool: &SqlitePool) -> Result<Vec<AppEvent>, sqlx::Error> {
    let mut events = get_events(pool).await?;
    events.retain(|e| e.dirty == Some(true));
    Ok(events)
}
pub async fn create_task(pool: &SqlitePool, task: AppTask) -> Result<(), sqlx::Error> {
    let status_str = task
        .status
        .map(|s| serde_json::to_string(&s).unwrap_or_default());
    let tags_str = task
        .tags
        .map(|t| serde_json::to_string(&t).unwrap_or_default());
    let subtasks_str = task
        .subtasks
        .map(|s| serde_json::to_string(&s).unwrap_or_default());
    let deps_str = task
        .dependencies
        .map(|d| serde_json::to_string(&d).unwrap_or_default());

    sqlx::query(
        "INSERT INTO tasks (
            id, title, status, priority, tags, subtasks, parent_task, dependencies, 
            est, added, canvas_x, canvas_y, on_canvas, remote_id, notes, tabs, 
            due, deleted, updated_at, etag, dirty
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(task.id)
    .bind(task.title)
    .bind(status_str)
    .bind(task.priority)
    .bind(tags_str)
    .bind(subtasks_str)
    .bind(task.parent_task)
    .bind(deps_str)
    .bind(task.est)
    .bind(task.added)
    .bind(task.canvas_x)
    .bind(task.canvas_y)
    .bind(task.on_canvas)
    .bind(task.remote_id)
    .bind(task.notes)
    .bind(task.tabs)
    .bind(task.due)
    .bind(task.deleted)
    .bind(task.updated_at)
    .bind(task.etag)
    .bind(task.dirty)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_task(pool: &SqlitePool, task: AppTask) -> Result<(), sqlx::Error> {
    let status_str = task
        .status
        .map(|s| serde_json::to_string(&s).unwrap_or_default());
    let tags_str = task
        .tags
        .map(|t| serde_json::to_string(&t).unwrap_or_default());
    let subtasks_str = task
        .subtasks
        .map(|s| serde_json::to_string(&s).unwrap_or_default());
    let deps_str = task
        .dependencies
        .map(|d| serde_json::to_string(&d).unwrap_or_default());

    sqlx::query(
        "UPDATE tasks SET 
            title = ?, status = ?, priority = ?, tags = ?, subtasks = ?, 
            parent_task = ?, dependencies = ?, est = ?, added = ?, canvas_x = ?, 
            canvas_y = ?, on_canvas = ?, remote_id = ?, notes = ?, tabs = ?, 
            due = ?, deleted = ?, updated_at = ?, etag = ?, dirty = ?
        WHERE id = ?",
    )
    .bind(task.title)
    .bind(status_str)
    .bind(task.priority)
    .bind(tags_str)
    .bind(subtasks_str)
    .bind(task.parent_task)
    .bind(deps_str)
    .bind(task.est)
    .bind(task.added)
    .bind(task.canvas_x)
    .bind(task.canvas_y)
    .bind(task.on_canvas)
    .bind(task.remote_id)
    .bind(task.notes)
    .bind(task.tabs)
    .bind(task.due)
    .bind(task.deleted)
    .bind(task.updated_at)
    .bind(task.etag)
    .bind(task.dirty)
    .bind(task.id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn upsert_task(pool: &SqlitePool, task: AppTask) -> Result<(), sqlx::Error> {
    let status_str = task
        .status
        .map(|s| serde_json::to_string(&s).unwrap_or_default());
    let tags_str = task
        .tags
        .map(|t| serde_json::to_string(&t).unwrap_or_default());
    let subtasks_str = task
        .subtasks
        .map(|s| serde_json::to_string(&s).unwrap_or_default());
    let deps_str = task
        .dependencies
        .map(|d| serde_json::to_string(&d).unwrap_or_default());

    sqlx::query(
        "INSERT INTO tasks (
            id, title, status, priority, tags, subtasks, parent_task, dependencies, 
            est, added, canvas_x, canvas_y, on_canvas, remote_id, notes, tabs, 
            due, deleted, updated_at, etag, dirty
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET 
            title = excluded.title, 
            status = excluded.status, 
            priority = excluded.priority, 
            tags = excluded.tags, 
            subtasks = excluded.subtasks, 
            parent_task = excluded.parent_task, 
            dependencies = excluded.dependencies, 
            est = excluded.est, 
            added = excluded.added, 
            canvas_x = excluded.canvas_x, 
            canvas_y = excluded.canvas_y, 
            on_canvas = excluded.on_canvas, 
            remote_id = excluded.remote_id, 
            notes = excluded.notes, 
            tabs = excluded.tabs, 
            due = excluded.due, 
            deleted = excluded.deleted, 
            updated_at = excluded.updated_at, 
            etag = excluded.etag,
            dirty = excluded.dirty",
    )
    .bind(task.id)
    .bind(task.title)
    .bind(status_str)
    .bind(task.priority)
    .bind(tags_str)
    .bind(subtasks_str)
    .bind(task.parent_task)
    .bind(deps_str)
    .bind(task.est)
    .bind(task.added)
    .bind(task.canvas_x)
    .bind(task.canvas_y)
    .bind(task.on_canvas)
    .bind(task.remote_id)
    .bind(task.notes)
    .bind(task.tabs)
    .bind(task.due)
    .bind(task.deleted)
    .bind(task.updated_at)
    .bind(task.etag)
    .bind(task.dirty)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_task(pool: &SqlitePool, id: String) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM tasks WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

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

pub async fn delete_event(pool: &SqlitePool, id: String) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM events WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_filtered_tasks(
    pool: &SqlitePool,
    filters: Vec<crate::domain::AppFilter>,
    sort: String,
    query_text: String,
) -> Result<Vec<AppTask>, sqlx::Error> {
    let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> =
        sqlx::QueryBuilder::new("SELECT * FROM tasks WHERE 1=1");

    for f in &filters {
        if let (Some(col), Some(val)) = (&f.column, &f.value) {
            let op = f.operator.as_deref().unwrap_or("is");
            let is_not = op == "is not";

            let values = if let Some(arr) = val.as_array() {
                arr.clone()
            } else {
                vec![val.clone()]
            };
            if values.is_empty() {
                continue;
            }

            if col == "status" {
                query_builder.push(if is_not { " AND status NOT IN (" } else { " AND status IN (" });
                let mut separated = query_builder.separated(", ");
                for v in &values {
                    if let Some(s) = v.as_str() {
                        separated.push_bind(format!("\"{}\"", s));
                    } else {
                        separated.push_bind(v.to_string());
                    }
                }
                query_builder.push(")");
            } else if col == "priority" {
                query_builder.push(if is_not { " AND priority NOT IN (" } else { " AND priority IN (" });
                let mut separated = query_builder.separated(", ");
                for v in &values {
                    if let Some(n) = v.as_i64() {
                        separated.push_bind(n as i32);
                    } else if let Some(s) = v.as_str() {
                        separated.push_bind(s.parse::<i32>().unwrap_or(-1));
                    }
                }
                query_builder.push(")");
            } else if col == "tag" {
                query_builder.push(" AND (");
                for (i, v) in values.iter().enumerate() {
                    if i > 0 {
                        query_builder.push(if is_not { " AND " } else { " OR " });
                    }
                    let vs = v.as_str().unwrap_or("");
                    let like_str = format!("%\"{}\"%", vs);
                    if is_not {
                        query_builder.push("(tags NOT LIKE ");
                        query_builder.push_bind(like_str);
                        query_builder.push(" OR tags IS NULL)");
                    } else {
                        query_builder.push("tags LIKE ");
                        query_builder.push_bind(like_str);
                    }
                }
                query_builder.push(")");
            }
        }
    }

    if !query_text.trim().is_empty() {
        let q = format!("%{}%", query_text.trim().to_lowercase());
        query_builder.push(" AND (LOWER(title) LIKE ");
        query_builder.push_bind(q.clone());
        query_builder.push(" OR LOWER(tags) LIKE ");
        query_builder.push_bind(q);
        query_builder.push(")");
    }

    query_builder.push(" ORDER BY ");
    if sort == "priority" {
        query_builder.push("COALESCE(priority, 0) DESC");
    } else if sort == "due" {
        query_builder.push("COALESCE(due, '9999') ASC");
    } else if sort == "title" {
        query_builder.push("title ASC");
    } else {
        query_builder.push("id ASC");
    }

    let rows = query_builder.build().fetch_all(pool).await?;
    let mut tasks = Vec::new();
    for row in rows {
        tasks.push(row_to_task(&row)?);
    }

    Ok(tasks)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[tokio::test]
    async fn test_init_db_in_memory() {
        let pool = init_db("sqlite::memory:")
            .await
            .expect("Failed to init in-memory db");

        let row: (i64,) = sqlx::query_as("SELECT count(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .expect("Failed to query tasks count");

        assert_eq!(row.0, 0);
    }

    #[tokio::test]
    async fn test_get_empty_tasks() {
        let pool = init_db("sqlite::memory:").await.expect("Failed to init db");
        let tasks = get_tasks(&pool).await.expect("Failed to get tasks");
        assert!(tasks.is_empty());
    }
}
