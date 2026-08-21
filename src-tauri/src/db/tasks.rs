use crate::domain::AppTask;
use sqlx::SqlitePool;

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_tasks(pool: &SqlitePool) -> Result<Vec<AppTask>, sqlx::Error> {
    sqlx::query_as::<_, AppTask>("SELECT t.id, t.title, t.status, t.priority, (SELECT CASE WHEN COUNT(tg.id) > 0 THEN json_group_array(json_object('id', tg.id, 'name', tg.name, 'color', tg.color)) ELSE 'null' END FROM task_tags tt JOIN tags tg ON tt.tag_id = tg.id WHERE tt.task_id = t.id) as tags, COALESCE(t.subtasks, 'null') as subtasks, t.parent_task, COALESCE(t.dependencies, 'null') as dependencies, t.est, t.added, t.canvas_x, t.canvas_y, t.on_canvas, t.remote_id, t.notes, t.tabs, t.due, t.deleted, t.updated_at, t.etag, t.dirty FROM tasks t")
        .fetch_all(pool)
        .await
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_task(pool: &SqlitePool, id: &str) -> Result<Option<AppTask>, sqlx::Error> {
    sqlx::query_as::<_, AppTask>("SELECT t.id, t.title, t.status, t.priority, (SELECT CASE WHEN COUNT(tg.id) > 0 THEN json_group_array(json_object('id', tg.id, 'name', tg.name, 'color', tg.color)) ELSE 'null' END FROM task_tags tt JOIN tags tg ON tt.tag_id = tg.id WHERE tt.task_id = t.id) as tags, COALESCE(t.subtasks, 'null') as subtasks, t.parent_task, COALESCE(t.dependencies, 'null') as dependencies, t.est, t.added, t.canvas_x, t.canvas_y, t.on_canvas, t.remote_id, t.notes, t.tabs, t.due, t.deleted, t.updated_at, t.etag, t.dirty FROM tasks t WHERE t.id = ?")
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

async fn sync_task_tags(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, task_id: &str, tags: Option<Vec<crate::domain::Tag>>) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM task_tags WHERE task_id = ?").bind(task_id).execute(&mut **tx).await?;
    if let Some(tags) = tags {
        for tag in tags {
            sqlx::query("INSERT INTO tags (id, name, color) VALUES (?, ?, ?) ON CONFLICT(name) DO UPDATE SET color = excluded.color, id = excluded.id")
                .bind(&tag.id)
                .bind(&tag.name)
                .bind(&tag.color)
                .execute(&mut **tx).await?;
            sqlx::query("INSERT INTO task_tags (task_id, tag_id) VALUES (?, ?)")
                .bind(task_id)
                .bind(&tag.id)
                .execute(&mut **tx).await?;
        }
    }
    Ok(())
}
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
    .bind(task.subtasks.as_ref().map(|s| sqlx::types::Json(s.clone())))
    .bind(&task.parent_task)
    .bind(task.dependencies.as_ref().map(|d| sqlx::types::Json(d.clone())))
    .bind(&task.est)
    .bind(&task.added)
    .bind(&task.canvas_x)
    .bind(&task.canvas_y)
    .bind(&task.on_canvas)
    .bind(&task.remote_id)
    .bind(&task.notes)
    .bind(&task.tabs)
    .bind(&task.due)
    .bind(&task.deleted)
    .bind(&task.updated_at)
    .bind(&task.etag)
    .bind(&task.dirty)
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
    .bind(task.subtasks.as_ref().map(|s| sqlx::types::Json(s.clone())))
    .bind(&task.parent_task)
    .bind(task.dependencies.as_ref().map(|d| sqlx::types::Json(d.clone())))
    .bind(&task.est)
    .bind(&task.added)
    .bind(&task.canvas_x)
    .bind(&task.canvas_y)
    .bind(&task.on_canvas)
    .bind(&task.remote_id)
    .bind(&task.notes)
    .bind(&task.tabs)
    .bind(&task.due)
    .bind(&task.deleted)
    .bind(&task.updated_at)
    .bind(&task.etag)
    .bind(&task.dirty)
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
    .bind(task.subtasks.as_ref().map(|s| sqlx::types::Json(s.clone())))
    .bind(&task.parent_task)
    .bind(task.dependencies.as_ref().map(|d| sqlx::types::Json(d.clone())))
    .bind(&task.est)
    .bind(&task.added)
    .bind(&task.canvas_x)
    .bind(&task.canvas_y)
    .bind(&task.on_canvas)
    .bind(&task.remote_id)
    .bind(&task.notes)
    .bind(&task.tabs)
    .bind(&task.due)
    .bind(&task.deleted)
    .bind(&task.updated_at)
    .bind(&task.etag)
    .bind(&task.dirty)
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

impl super::FilterColumnExt for crate::domain::TaskFilterColumn {
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
                    if is_not {
                        builder.push("t.id NOT IN (SELECT task_id FROM task_tags tt JOIN tags tg ON tt.tag_id = tg.id WHERE tg.name LIKE ");
                        builder.push_bind(format!("%{vs}%"));
                        builder.push(")");
                    } else {
                        builder.push("t.id IN (SELECT task_id FROM task_tags tt JOIN tags tg ON tt.tag_id = tg.id WHERE tg.name LIKE ");
                        builder.push_bind(format!("%{vs}%"));
                        builder.push(")");
                    }
                }
                builder.push(")");
            }
        }
    }
}

use super::FilterColumnExt;

pub async fn get_filtered_tasks(
    pool: &SqlitePool,
    filters: Vec<crate::domain::AppTaskFilter>,
    sort: String,
    query_text: String,
) -> Result<Vec<AppTask>, sqlx::Error> {
    let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> =
        sqlx::QueryBuilder::new("SELECT t.id, t.title, t.status, t.priority, (SELECT CASE WHEN COUNT(tg.id) > 0 THEN json_group_array(json_object('id', tg.id, 'name', tg.name, 'color', tg.color)) ELSE 'null' END FROM task_tags tt JOIN tags tg ON tt.tag_id = tg.id WHERE tt.task_id = t.id) as tags, COALESCE(t.subtasks, 'null') as subtasks, t.parent_task, COALESCE(t.dependencies, 'null') as dependencies, t.est, t.added, t.canvas_x, t.canvas_y, t.on_canvas, t.remote_id, t.notes, t.tabs, t.due, t.deleted, t.updated_at, t.etag, t.dirty FROM tasks t WHERE 1=1");

    for f in &filters {
        if let (Some(col), Some(val)) = (&f.column, &f.value) {
            let op = f.operator.as_deref().unwrap_or("is");
            col.apply_sql(&mut query_builder, op, val);
        }
    }

    if !query_text.trim().is_empty() {
        let q = format!("%{}%", query_text.trim().to_lowercase());
        query_builder.push(" AND (LOWER(t.title) LIKE ");
        query_builder.push_bind(q.clone());
        query_builder.push(" OR t.id IN (SELECT task_id FROM task_tags tt JOIN tags tg ON tt.tag_id = tg.id WHERE LOWER(tg.name) LIKE ");
        query_builder.push_bind(q);
        query_builder.push("))");
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


