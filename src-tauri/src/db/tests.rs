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
async fn test_schema_creates_all_tables() {
    let pool = init_db("sqlite::memory:").await.expect("Failed to init db");

    for table in [
        "tasks",
        "events",
        "settings",
        "ui_state",
        "sync_queue",
        "tags",
        "task_tags",
    ] {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
                .bind(table)
                .fetch_one(&pool)
                .await
                .expect("Failed to query sqlite_master");
        assert_eq!(row.0, 1, "table {table} should exist after init_db");
    }
}

#[tokio::test]
async fn test_settings_roundtrip_and_overwrite() {
    let pool = init_db("sqlite::memory:").await.expect("Failed to init db");

    let key = "test.roundtrip";

    assert_eq!(get_setting(&pool, key).await.expect("get failed"), None);

    set_setting(&pool, key, r#"[{"column":"status"}]"#)
        .await
        .expect("set failed");
    assert_eq!(
        get_setting(&pool, key)
            .await
            .expect("get failed")
            .as_deref(),
        Some(r#"[{"column":"status"}]"#)
    );

    set_setting(&pool, key, "[]")
        .await
        .expect("overwrite failed");
    assert_eq!(
        get_setting(&pool, key)
            .await
            .expect("get failed")
            .as_deref(),
        Some("[]")
    );

    delete_setting(&pool, key).await.expect("delete failed");
    assert_eq!(get_setting(&pool, key).await.expect("get failed"), None);
}

#[tokio::test]
async fn test_ui_state_roundtrip() {
    let pool = init_db("sqlite::memory:").await.expect("Failed to init db");
    let key = "ui.task_list.filters";
    let value = r#"[{"column":"status","value":["done"]}]"#;

    assert_eq!(get_ui_state(&pool, key).await.expect("get failed"), None);

    set_ui_state(&pool, key, value).await.expect("set failed");
    assert_eq!(
        get_ui_state(&pool, key)
            .await
            .expect("get failed")
            .as_deref(),
        Some(value)
    );
}

#[tokio::test]
async fn test_init_db_migrates_legacy_ui_settings_rows_on_reopen() {
    let path = std::env::temp_dir().join(format!("taskroot-migration-{}.db", uuid::Uuid::new_v4()));
    let url = format!("sqlite:{}", path.to_string_lossy());

    let pool = init_db(&url).await.expect("Failed to init db");
    set_setting(&pool, "ui.task_list.sort", r#""title""#)
        .await
        .expect("seed failed");
    pool.close().await;

    // Reopening goes through `init_db`, which runs the migration boot path.
    let reopened = init_db(&url)
        .await
        .expect("Failed to reopen db with legacy row");

    assert_eq!(
        get_setting(&reopened, "ui.task_list.sort")
            .await
            .expect("get failed"),
        None
    );
    assert_eq!(
        get_ui_state(&reopened, "ui.task_list.sort")
            .await
            .expect("get failed")
            .as_deref(),
        Some(r#""title""#)
    );

    reopened.close().await;
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("db-wal"));
    let _ = std::fs::remove_file(path.with_extension("db-shm"));
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
        checklist: None,
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
        updated_at: Some("2026-08-26T12:00:00Z".into()),
        etag: Some("etag-1".into()),
        dirty: Some(true),
        task_list_id: None,
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
    use crate::domain::EventStatus;

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
        status: Some(EventStatus::Confirmed),
        updated_at: Some("2026-08-26T12:00:00Z".into()),
        color: None,
        etag: None,
        dirty: Some(true),
        is_all_day: Some(false),
        timezone: None,
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
    updated.status = Some(EventStatus::Cancelled);
    update_event(&pool, updated).await.expect("update failed");

    let refetched = get_event(&pool, "event-1")
        .await
        .expect("refetch failed")
        .expect("event missing after update");
    assert_eq!(refetched.title, "Standup moved");
    assert_eq!(refetched.status, Some(EventStatus::Cancelled));

    delete_event(&pool, "event-1".to_string())
        .await
        .expect("delete failed");
    assert!(get_event(&pool, "event-1")
        .await
        .expect("final fetch failed")
        .is_none());
}

#[tokio::test]
async fn test_resolve_calendar_color_uses_event_calendar_then_primary() {
    use crate::domain::{AppCalendar, CollectionId, Color};

    let pool = init_db("sqlite::memory:").await.expect("Failed to init db");
    let calendar = |id: &str, color: &str, primary: bool| AppCalendar {
        id: id.into(),
        summary: id.into(),
        color: Some(Color::try_from(color.to_string()).expect("valid color")),
        is_primary: Some(primary),
        access_role: None,
    };
    upsert_calendar(&pool, calendar("primary-cal", "#ff0000", true))
        .await
        .expect("seed primary failed");
    upsert_calendar(&pool, calendar("work", "#00ff00", false))
        .await
        .expect("seed work failed");

    let work = CollectionId::from("work");
    assert_eq!(
        resolve_calendar_color(&pool, Some(&work))
            .await
            .expect("resolve failed"),
        Some(Color::try_from("#00ff00".to_string()).expect("valid color"))
    );

    let unknown = CollectionId::from("unknown");
    assert_eq!(
        resolve_calendar_color(&pool, Some(&unknown))
            .await
            .expect("resolve failed"),
        Some(Color::try_from("#ff0000".to_string()).expect("valid color"))
    );
    assert_eq!(
        resolve_calendar_color(&pool, None)
            .await
            .expect("resolve failed"),
        Some(Color::try_from("#ff0000".to_string()).expect("valid color"))
    );
}
