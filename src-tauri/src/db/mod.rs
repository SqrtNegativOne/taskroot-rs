pub mod events;
pub mod settings;
pub mod tasks;

pub use events::*;
pub use settings::*;
pub use tasks::*;

use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;

/// # Errors
///
/// Returns an error if the operation fails.
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
