use crate::db;
use crate::domain;
use crate::error::AppError;
use crate::sync;

#[tauri::command]
#[allow(clippy::needless_pass_by_value, clippy::must_use_candidate)]
pub fn parse_sigils(task_name: String) -> domain::ParsedSigils {
    domain::parse_sigils(&task_name)
}



/// # Errors
///
/// Returns an error if the database is unavailable or the write fails.
#[tauri::command]
pub async fn create_task(app: tauri::AppHandle, mut task: domain::AppTask) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;
    sync::push::push_or_enqueue(&app, &mut task, sync::types::SyncAction::Create).await;
    Ok(db::create_task(&pool, task).await?)
}

/// # Errors
///
/// Returns an error if the database is unavailable or the write fails.
#[tauri::command]
pub async fn update_task(app: tauri::AppHandle, mut task: domain::AppTask) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;
    sync::push::push_or_enqueue(&app, &mut task, sync::types::SyncAction::Update).await;
    Ok(db::update_task(&pool, task).await?)
}

/// # Errors
///
/// Returns an error if the database is unavailable or the delete fails.
#[tauri::command]
pub async fn delete_task(app: tauri::AppHandle, id: String) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;

    if let Ok(Some(task)) = db::get_task(&pool, &id).await {
        let app_clone = app.clone();
        tokio::spawn(async move {
            sync::push::push_delete_or_enqueue(&app_clone, &task).await;
        });
    }

    Ok(db::delete_task(&pool, id).await?)
}

#[tauri::command]
pub async fn get_past_due_task_ids(app: tauri::AppHandle) -> Result<Vec<String>, AppError> {
    let pool = crate::db_pool(&app)?;
    let now_iso = chrono::Utc::now().to_rfc3339();
    let ids = sqlx::query_scalar::<_, String>(
        r"
        SELECT DISTINCT t.id 
        FROM tasks t 
        JOIN events e ON e.task_id = t.id 
        WHERE t.status NOT IN ('done', 'cancelled')
        AND e.end_time < ?
        "
    )
    .bind(now_iso)
    .fetch_all(&*pool)
    .await
    .map_err(|e| AppError::Internal(format!("Database error: {e}")))?;
    
    Ok(ids)
}

