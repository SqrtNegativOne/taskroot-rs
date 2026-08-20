use crate::domain::AppTask;
use sqlx::SqlitePool;

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_tasks(pool: &SqlitePool) -> Result<Vec<AppTask>, sqlx::Error> {
    sqlx::query_as::<_, AppTask>("SELECT * FROM tasks")
        .fetch_all(pool)
        .await
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_task(pool: &SqlitePool, id: &str) -> Result<Option<AppTask>, sqlx::Error> {
    sqlx::query_as::<_, AppTask>("SELECT * FROM tasks WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
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
    sqlx::query(
        "INSERT INTO tasks (
            id, title, status, priority, tags, subtasks, parent_task, dependencies, 
            est, added, canvas_x, canvas_y, on_canvas, remote_id, notes, tabs, 
            due, deleted, updated_at, etag, dirty
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(task.id)
    .bind(task.title)
    .bind(task.status)
    .bind(task.priority)
    .bind(task.tags.map(sqlx::types::Json))
    .bind(task.subtasks.map(sqlx::types::Json))
    .bind(task.parent_task)
    .bind(task.dependencies.map(sqlx::types::Json))
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
    sqlx::query(
        "UPDATE tasks SET 
            title = ?, status = ?, priority = ?, tags = ?, subtasks = ?, 
            parent_task = ?, dependencies = ?, est = ?, added = ?, canvas_x = ?, 
            canvas_y = ?, on_canvas = ?, remote_id = ?, notes = ?, tabs = ?, 
            due = ?, deleted = ?, updated_at = ?, etag = ?, dirty = ?
        WHERE id = ?",
    )
    .bind(task.title)
    .bind(task.status)
    .bind(task.priority)
    .bind(task.tags.map(sqlx::types::Json))
    .bind(task.subtasks.map(sqlx::types::Json))
    .bind(task.parent_task)
    .bind(task.dependencies.map(sqlx::types::Json))
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
    .bind(task.status)
    .bind(task.priority)
    .bind(task.tags.map(sqlx::types::Json))
    .bind(task.subtasks.map(sqlx::types::Json))
    .bind(task.parent_task)
    .bind(task.dependencies.map(sqlx::types::Json))
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
pub trait FilterColumnExt {
    fn apply_sql(&self, builder: &mut sqlx::QueryBuilder<sqlx::Sqlite>, op: &str, val: &serde_json::Value);
}

impl FilterColumnExt for crate::domain::FilterColumn {
    fn apply_sql(&self, builder: &mut sqlx::QueryBuilder<sqlx::Sqlite>, op: &str, val: &serde_json::Value) {
        let is_not = op == "is not";
        let values = val.as_array().map_or_else(|| vec![val.clone()], std::clone::Clone::clone);
        if values.is_empty() {
            return;
        }

        match self {
            Self::Status => {
                builder.push(if is_not { " AND status NOT IN (" } else { " AND status IN (" });
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
            Self::Priority => {
                builder.push(if is_not { " AND priority NOT IN (" } else { " AND priority IN (" });
                let mut separated = builder.separated(", ");
                for v in &values {
                    if let Some(n) = v.as_i64() {
                        separated.push_bind(i32::try_from(n).unwrap_or(0));
                    } else if let Some(s) = v.as_str() {
                        separated.push_bind(s.parse::<i32>().unwrap_or(-1));
                    }
                }
                builder.push(")");
            }
            Self::Tag => {
                builder.push(" AND (");
                for (i, v) in values.iter().enumerate() {
                    if i > 0 {
                        builder.push(if is_not { " AND " } else { " OR " });
                    }
                    let vs = v.as_str().unwrap_or("");
                    let like_str = format!("%\"{vs}\"%");
                    if is_not {
                        builder.push("(tags NOT LIKE ");
                        builder.push_bind(like_str);
                        builder.push(" OR tags IS NULL)");
                    } else {
                        builder.push("tags LIKE ");
                        builder.push_bind(like_str);
                    }
                }
                builder.push(")");
            }
        }
    }
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
            col.apply_sql(&mut query_builder, op, val);
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

    let tasks = query_builder.build_query_as::<AppTask>().fetch_all(pool).await?;

    Ok(tasks)
}


