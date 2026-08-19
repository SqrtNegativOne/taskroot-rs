use crate::domain::AppTask;
use sqlx::{Row, SqlitePool};

pub fn row_to_task(row: &sqlx::sqlite::SqliteRow) -> Result<AppTask, sqlx::Error> {
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

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_tasks(pool: &SqlitePool) -> Result<Vec<AppTask>, sqlx::Error> {
    let rows = sqlx::query("SELECT * FROM tasks").fetch_all(pool).await?;
    let mut tasks = Vec::new();

    for row in rows {
        tasks.push(row_to_task(&row)?);
    }

    Ok(tasks)
}

/// # Errors
///
/// Returns an error if the operation fails.
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

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_dirty_tasks(pool: &SqlitePool) -> Result<Vec<AppTask>, sqlx::Error> {
    let mut tasks = get_tasks(pool).await?;
    tasks.retain(|t| t.dirty == Some(true));
    Ok(tasks)
}

/// # Errors
///
/// Returns an error if the operation fails.
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

/// # Errors
///
/// Returns an error if the operation fails.
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

/// # Errors
///
/// Returns an error if the operation fails.
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

/// # Errors
///
/// Returns an error if the operation fails.
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

            let values = val.as_array().map_or_else(|| vec![val.clone()], std::clone::Clone::clone);
            if values.is_empty() {
                continue;
            }

            if col == "status" {
                query_builder.push(if is_not { " AND status NOT IN (" } else { " AND status IN (" });
                let mut separated = query_builder.separated(", ");
                for v in &values {
                    if let Some(s) = v.as_str() {
                        separated.push_bind(format!("\"{s}\""));
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
                        separated.push_bind(i32::try_from(n).unwrap_or(0));
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
                    let like_str = format!("%\"{vs}\"%");
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
