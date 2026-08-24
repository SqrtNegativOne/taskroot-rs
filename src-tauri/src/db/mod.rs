pub mod events;
pub mod settings;
pub mod task_filters;
pub mod tasks;

pub use events::*;
pub use settings::*;
pub use task_filters::*;
pub use tasks::*;

use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;

pub trait FilterColumnExt {
    fn apply_sql(
        &self,
        builder: &mut sqlx::QueryBuilder<sqlx::Sqlite>,
        op: &str,
        val: &serde_json::Value,
    );
}

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// # Errors
///
/// Returns an error if connecting or running migrations fails.
pub async fn init_db(db_path: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(db_path)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePool::connect_with(options).await?;
    MIGRATOR.run(&pool).await?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::domain::{AppEvent, AppTask, AppTaskStatus, Tag, TaskPriority};

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

    #[tokio::test]
    async fn test_migrations_create_all_tables() {
        let pool = init_db("sqlite::memory:").await.expect("Failed to init db");

        let applied: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM _sqlx_migrations WHERE version = 1")
                .fetch_one(&pool)
                .await
                .expect("Failed to query migrations table");
        assert_eq!(applied.0, 1);

        for table in [
            "tasks",
            "events",
            "settings",
            "sync_queue",
            "tags",
            "task_tags",
        ] {
            let row: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
            )
            .bind(table)
            .fetch_one(&pool)
            .await
            .expect("Failed to query sqlite_master");
            assert_eq!(row.0, 1, "table {table} should exist after migrations");
        }
    }

    #[tokio::test]
    async fn test_task_crud_roundtrip() {
        let pool = init_db("sqlite::memory:").await.expect("Failed to init db");
        let task = AppTask {
            id: "task-1".into(),
            title: "Write report".into(),
            status: Some(AppTaskStatus::Todo),
            priority: Some(TaskPriority::Medium),
            tags: Some(vec![Tag {
                id: "tag-1".into(),
                name: "work".into(),
                color: None,
            }]),
            subtasks: None,
            parent_task: None,
            dependencies: Some(vec!["task-0".into()]),
            est: Some(45),
            added: Some("2026-08-24".into()),
            canvas_x: Some(1.5),
            canvas_y: Some(-2.0),
            on_canvas: Some(true),
            remote_id: None,
            notes: Some("notes".into()),
            tabs: None,
            due: Some("2026-08-25".into()),
            deleted: None,
            updated_at: Some(7),
            etag: Some("etag-1".into()),
            dirty: Some(true),
        };

        create_task(&pool, task.clone())
            .await
            .expect("insert failed");

        let fetched = get_task(&pool, "task-1")
            .await
            .expect("fetch failed")
            .expect("task missing");
        assert_eq!(fetched.title, task.title);
        assert_eq!(fetched.status, Some(AppTaskStatus::Todo));
        assert_eq!(fetched.priority, Some(TaskPriority::Medium));
        assert_eq!(
            fetched.tags,
            Some(vec![Tag {
                id: "tag-1".into(),
                name: "work".into(),
                color: None
            }])
        );
        assert_eq!(fetched.dependencies, Some(vec!["task-0".into()]));
        assert_eq!(fetched.est, Some(45));
        assert_eq!(fetched.canvas_x, Some(1.5));
        assert_eq!(fetched.on_canvas, Some(true));
        assert_eq!(fetched.due.as_deref(), Some("2026-08-25"));
        assert_eq!(fetched.dirty, Some(true));

        let mut updated = fetched;
        updated.title = "Write report v2".into();
        updated.status = Some(AppTaskStatus::Doing);
        updated.dirty = Some(false);
        update_task(&pool, updated).await.expect("update failed");

        let refetched = get_task(&pool, "task-1")
            .await
            .expect("refetch failed")
            .expect("task missing after update");
        assert_eq!(refetched.title, "Write report v2");
        assert_eq!(refetched.status, Some(AppTaskStatus::Doing));
        assert_eq!(refetched.dirty, Some(false));

        delete_task(&pool, "task-1".into())
            .await
            .expect("delete failed");
        assert!(get_task(&pool, "task-1")
            .await
            .expect("final fetch failed")
            .is_none());
    }

    #[tokio::test]
    async fn test_event_crud_roundtrip() {
        let pool = init_db("sqlite::memory:").await.expect("Failed to init db");
        let event = AppEvent {
            id: "event-1".into(),
            remote_id: Some("remote-1".into()),
            remote_collection_id: Some("cal-1".into()),
            task_id: None,
            title: "Standup".into(),
            description: None,
            start_time: "2026-08-24T09:00:00".into(),
            end_time: "2026-08-24T09:15:00".into(),
            rrule: None,
            exdates: Some(vec!["2026-08-25".into()]),
            recurring_event_id: None,
            original_start_time: None,
            cancelled: Some(false),
            updated_at: Some(7),
            color: None,
            deleted: None,
            etag: None,
            dirty: Some(true),
        };

        create_event(&pool, event.clone())
            .await
            .expect("insert failed");

        let fetched = get_event(&pool, "event-1")
            .await
            .expect("fetch failed")
            .expect("event missing");
        assert_eq!(fetched.title, event.title);
        assert_eq!(fetched.start_time, event.start_time);
        assert_eq!(fetched.exdates, event.exdates);
        assert_eq!(fetched.dirty, Some(true));

        let mut updated = fetched;
        updated.title = "Standup moved".into();
        updated.cancelled = Some(true);
        update_event(&pool, updated).await.expect("update failed");

        let refetched = get_event(&pool, "event-1")
            .await
            .expect("refetch failed")
            .expect("event missing after update");
        assert_eq!(refetched.title, "Standup moved");
        assert_eq!(refetched.cancelled, Some(true));

        delete_event(&pool, "event-1".to_string())
            .await
            .expect("delete failed");
        assert!(get_event(&pool, "event-1")
            .await
            .expect("final fetch failed")
            .is_none());
    }
}
