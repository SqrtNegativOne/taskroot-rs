use crate::db;
use crate::domain;
use crate::error::AppError;

#[tauri::command]
#[allow(clippy::needless_pass_by_value, clippy::must_use_candidate)]
pub fn parse_sigils(task_name: String) -> domain::ParsedSigils {
    domain::parse_sigils(&task_name)
}

/// # Errors
///
/// Returns an error if the database is unavailable or the write fails.
#[tauri::command]
pub async fn create_task(app: tauri::AppHandle, task: domain::AppTask) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;
    Ok(db::create_task(&pool, task).await?)
}

/// # Errors
///
/// Returns an error if the database is unavailable or the write fails.
#[tauri::command]
pub async fn update_task(app: tauri::AppHandle, task: domain::AppTask) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;
    Ok(db::update_task(&pool, task).await?)
}

/// # Errors
///
/// Returns an error if the database is unavailable or the delete fails.
#[tauri::command]
pub async fn delete_task(app: tauri::AppHandle, id: String) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;
    Ok(db::delete_task(&pool, id).await?)
}

/// Ids of open tasks whose linked event has already ended.
///
/// The `get_past_due_task_ids` command body. The query stays here rather than in
/// `db` because its failure keeps the `internal` error code the frontend sees
/// today (`db::*` failures map to `db`).
pub(crate) async fn past_due_task_ids(pool: &sqlx::SqlitePool) -> Result<Vec<String>, AppError> {
    let now_iso = chrono::Utc::now().to_rfc3339();
    sqlx::query_scalar::<_, String>(
        r"
        SELECT DISTINCT t.id 
        FROM tasks t 
        JOIN events e ON e.task_id = t.id 
        WHERE t.status NOT IN ('done', 'cancelled')
        AND e.end_time < ?
        ",
    )
    .bind(now_iso)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Database error: {e}")))
}

#[tauri::command]
pub async fn get_past_due_task_ids(app: tauri::AppHandle) -> Result<Vec<String>, AppError> {
    let pool = crate::db_pool(&app)?;
    past_due_task_ids(&pool).await
}
