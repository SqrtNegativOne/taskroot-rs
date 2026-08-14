use sqlx::{sqlite::SqliteConnectOptions, SqlitePool, Row};
use std::str::FromStr;
use crate::domain::{AppTask, AppEvent};

pub async fn init_db(db_path: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(db_path)?
        .create_if_missing(true);
        
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
            etag TEXT
        );"
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
            etag TEXT
        );"
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

pub async fn get_tasks(pool: &SqlitePool) -> Result<Vec<AppTask>, sqlx::Error> {
    let rows = sqlx::query("SELECT * FROM tasks").fetch_all(pool).await?;
    let mut tasks = Vec::new();
    
    for row in rows {
        let status_str: Option<String> = row.try_get("status")?;
        let tags_str: Option<String> = row.try_get("tags")?;
        let subtasks_str: Option<String> = row.try_get("subtasks")?;
        let dependencies_str: Option<String> = row.try_get("dependencies")?;

        tasks.push(AppTask {
            id: row.try_get("id")?,
            title: row.try_get("title")?,
            status: status_str.and_then(|s| serde_json::from_value(serde_json::Value::String(s)).ok()),
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
        });
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
            event_type: serde_json::from_value(serde_json::Value::String(type_str)).unwrap_or(crate::domain::AppEventType::Info),
            rrule: row.try_get("rrule")?,
            exdates: exdates_str.and_then(|s| serde_json::from_str(&s).ok()),
            recurring_event_id: row.try_get("recurring_event_id")?,
            original_start_time: row.try_get("original_start_time")?,
            cancelled: row.try_get("cancelled")?,
            updated_at: row.try_get("updated_at")?,
            color: None, // We don't have it in db yet
            deleted: row.try_get("deleted")?,
            etag: row.try_get("etag")?,
        });
    }

    Ok(events)
}
pub async fn create_task(pool: &SqlitePool, task: AppTask) -> Result<(), sqlx::Error> {
    let status_str = task.status.map(|s| serde_json::to_string(&s).unwrap_or_default());
    let tags_str = task.tags.map(|t| serde_json::to_string(&t).unwrap_or_default());
    let subtasks_str = task.subtasks.map(|s| serde_json::to_string(&s).unwrap_or_default());
    let deps_str = task.dependencies.map(|d| serde_json::to_string(&d).unwrap_or_default());

    sqlx::query(
        "INSERT INTO tasks (
            id, title, status, priority, tags, subtasks, parent_task, dependencies, 
            est, added, canvas_x, canvas_y, on_canvas, remote_id, notes, tabs, 
            due, deleted, updated_at, etag
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
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
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn update_task(pool: &SqlitePool, task: AppTask) -> Result<(), sqlx::Error> {
    let status_str = task.status.map(|s| serde_json::to_string(&s).unwrap_or_default());
    let tags_str = task.tags.map(|t| serde_json::to_string(&t).unwrap_or_default());
    let subtasks_str = task.subtasks.map(|s| serde_json::to_string(&s).unwrap_or_default());
    let deps_str = task.dependencies.map(|d| serde_json::to_string(&d).unwrap_or_default());

    sqlx::query(
        "UPDATE tasks SET 
            title = ?, status = ?, priority = ?, tags = ?, subtasks = ?, 
            parent_task = ?, dependencies = ?, est = ?, added = ?, canvas_x = ?, 
            canvas_y = ?, on_canvas = ?, remote_id = ?, notes = ?, tabs = ?, 
            due = ?, deleted = ?, updated_at = ?, etag = ?
        WHERE id = ?"
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
    .bind(task.id)
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
    let exdates_str = event.exdates.map(|e| serde_json::to_string(&e).unwrap_or_default());

    sqlx::query(
        "INSERT INTO events (
            id, remote_id, remote_collection_id, task_id, title, description, 
            start_time, end_time, event_type, rrule, exdates, recurring_event_id, 
            original_start_time, cancelled, updated_at, deleted, etag
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
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
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn update_event(pool: &SqlitePool, event: AppEvent) -> Result<(), sqlx::Error> {
    let type_str = serde_json::to_string(&event.event_type).unwrap_or_default();
    let exdates_str = event.exdates.map(|e| serde_json::to_string(&e).unwrap_or_default());

    sqlx::query(
        "UPDATE events SET 
            remote_id = ?, remote_collection_id = ?, task_id = ?, title = ?, 
            description = ?, start_time = ?, end_time = ?, event_type = ?, 
            rrule = ?, exdates = ?, recurring_event_id = ?, original_start_time = ?, 
            cancelled = ?, updated_at = ?, deleted = ?, etag = ?
        WHERE id = ?"
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
    .bind(event.id)
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[tokio::test]
    async fn test_init_db_in_memory() {
        let pool = init_db("sqlite::memory:").await.expect("Failed to init in-memory db");
        
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
