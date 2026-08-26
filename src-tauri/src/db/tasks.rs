use crate::domain::{AppTask, Tag};
use sqlx::SqlitePool;

macro_rules! task_select_sql {
    ($suffix:literal) => {
        concat!(
            "SELECT t.id, t.title, t.status, t.priority, ",
            "(SELECT CASE WHEN COUNT(tg.id) > 0 THEN json_group_array(json_object('id', tg.id, 'name', tg.name, 'color', tg.color)) ELSE 'null' END ",
            "FROM task_tags tt JOIN tags tg ON tt.tag_id = tg.id WHERE tt.task_id = t.id) as tags, ",
            "COALESCE(t.subtasks, 'null') as checklist, t.parent_task, COALESCE(t.dependencies, 'null') as dependencies, ",
            "t.est, t.added, t.canvas_x, t.canvas_y, t.on_canvas, t.remote_id, t.notes, t.tabs, t.due, t.deleted, t.updated_at, t.etag, t.dirty",
            " FROM tasks t",
            $suffix
        )
    };
}

pub(crate) use task_select_sql;

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_tasks(pool: &SqlitePool) -> Result<Vec<AppTask>, sqlx::Error> {
    sqlx::query_as::<_, AppTask>(task_select_sql!(""))
        .fetch_all(pool)
        .await
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_task(pool: &SqlitePool, id: &str) -> Result<Option<AppTask>, sqlx::Error> {
    sqlx::query_as::<_, AppTask>(task_select_sql!(" WHERE t.id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Dirty-task feed for the offline-enqueue roadmap (see TODO.md).
#[allow(dead_code)]
/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_dirty_tasks(pool: &SqlitePool) -> Result<Vec<AppTask>, sqlx::Error> {
    let mut tasks = get_tasks(pool).await?;
    tasks.retain(|t| t.dirty == Some(true));
    Ok(tasks)
}

async fn sync_task_tags(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    task_id: &str,
    tags: Option<Vec<Tag>>,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM task_tags WHERE task_id = ?")
        .bind(task_id)
        .execute(&mut **tx)
        .await?;
    if let Some(tags) = tags {
        for tag in tags {
            sqlx::query("INSERT INTO tags (id, name, color) VALUES (?, ?, ?) ON CONFLICT(name) DO UPDATE SET color = excluded.color, id = excluded.id")
                .bind(&tag.id)
                .bind(&tag.name)
                .bind(&tag.color)
                .execute(&mut **tx)
                .await?;
            sqlx::query("INSERT INTO task_tags (task_id, tag_id) VALUES (?, ?)")
                .bind(task_id)
                .bind(&tag.id)
                .execute(&mut **tx)
                .await?;
        }
    }
    Ok(())
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn create_task(pool: &SqlitePool, task: AppTask) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO tasks (
            id, title, status, priority, subtasks, parent_task, dependencies,
            est, added, canvas_x, canvas_y, on_canvas, remote_id, notes, tabs,
            due, deleted, updated_at, etag, dirty
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&task.id)
    .bind(&task.title)
    .bind(&task.status)
    .bind(&task.priority)
    .bind(task.checklist.as_ref().map(|s| sqlx::types::Json(s.clone())))
    .bind(&task.parent_task)
    .bind(
        task.dependencies
            .as_ref()
            .map(|d| sqlx::types::Json(d.clone())),
    )
    .bind(task.est)
    .bind(&task.added)
    .bind(task.canvas_x)
    .bind(task.canvas_y)
    .bind(task.on_canvas)
    .bind(&task.remote_id)
    .bind(&task.notes)
    .bind(&task.tabs)
    .bind(&task.due)
    .bind(task.deleted)
    .bind(task.updated_at)
    .bind(&task.etag)
    .bind(task.dirty)
    .execute(&mut *tx)
    .await?;

    sync_task_tags(&mut tx, &task.id, task.tags).await?;
    tx.commit().await?;
    Ok(())
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn update_task(pool: &SqlitePool, task: AppTask) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE tasks SET
            title = ?, status = ?, priority = ?, subtasks = ?,
            parent_task = ?, dependencies = ?, est = ?, added = ?, canvas_x = ?,
            canvas_y = ?, on_canvas = ?, remote_id = ?, notes = ?, tabs = ?,
            due = ?, deleted = ?, updated_at = ?, etag = ?, dirty = ?
        WHERE id = ?",
    )
    .bind(&task.title)
    .bind(&task.status)
    .bind(&task.priority)
    .bind(task.checklist.as_ref().map(|s| sqlx::types::Json(s.clone())))
    .bind(&task.parent_task)
    .bind(
        task.dependencies
            .as_ref()
            .map(|d| sqlx::types::Json(d.clone())),
    )
    .bind(task.est)
    .bind(&task.added)
    .bind(task.canvas_x)
    .bind(task.canvas_y)
    .bind(task.on_canvas)
    .bind(&task.remote_id)
    .bind(&task.notes)
    .bind(&task.tabs)
    .bind(&task.due)
    .bind(task.deleted)
    .bind(task.updated_at)
    .bind(&task.etag)
    .bind(task.dirty)
    .bind(&task.id)
    .execute(&mut *tx)
    .await?;

    sync_task_tags(&mut tx, &task.id, task.tags).await?;
    tx.commit().await?;
    Ok(())
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn upsert_task(pool: &SqlitePool, task: AppTask) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO tasks (
            id, title, status, priority, subtasks, parent_task, dependencies,
            est, added, canvas_x, canvas_y, on_canvas, remote_id, notes, tabs,
            due, deleted, updated_at, etag, dirty
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            status = excluded.status,
            priority = excluded.priority,
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
    .bind(&task.id)
    .bind(&task.title)
    .bind(&task.status)
    .bind(&task.priority)
    .bind(task.checklist.as_ref().map(|s| sqlx::types::Json(s.clone())))
    .bind(&task.parent_task)
    .bind(
        task.dependencies
            .as_ref()
            .map(|d| sqlx::types::Json(d.clone())),
    )
    .bind(task.est)
    .bind(&task.added)
    .bind(task.canvas_x)
    .bind(task.canvas_y)
    .bind(task.on_canvas)
    .bind(&task.remote_id)
    .bind(&task.notes)
    .bind(&task.tabs)
    .bind(&task.due)
    .bind(task.deleted)
    .bind(task.updated_at)
    .bind(&task.etag)
    .bind(task.dirty)
    .execute(&mut *tx)
    .await?;

    sync_task_tags(&mut tx, &task.id, task.tags).await?;
    tx.commit().await?;
    Ok(())
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn delete_task(pool: &SqlitePool, id: String) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM tasks WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
