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
/// Returns an error if the database is unavailable or the query fails.
#[tauri::command]
pub async fn get_tasks(app: tauri::AppHandle) -> Result<Vec<domain::AppTask>, AppError> {
    let pool = crate::db_pool(&app)?;
    Ok(db::get_tasks(&pool).await?)
}

/// # Errors
///
/// Returns an error if the database is unavailable or the write fails.
#[tauri::command]
pub async fn create_task(app: tauri::AppHandle, mut task: domain::AppTask) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;
    sync::push::push_or_enqueue(&pool, &mut task, sync::types::SyncAction::Create).await;
    Ok(db::create_task(&pool, task).await?)
}

/// # Errors
///
/// Returns an error if the database is unavailable or the write fails.
#[tauri::command]
pub async fn update_task(app: tauri::AppHandle, mut task: domain::AppTask) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;
    sync::push::push_or_enqueue(&pool, &mut task, sync::types::SyncAction::Update).await;
    Ok(db::update_task(&pool, task).await?)
}

/// # Errors
///
/// Returns an error if the database is unavailable or the delete fails.
#[tauri::command]
pub async fn delete_task(app: tauri::AppHandle, id: String) -> Result<(), AppError> {
    let pool = crate::db_pool(&app)?;

    if let Ok(Some(task)) = db::get_task(&pool, &id).await {
        sync::push::push_delete_or_enqueue(&pool, &task).await;
    }

    Ok(db::delete_task(&pool, id).await?)
}
