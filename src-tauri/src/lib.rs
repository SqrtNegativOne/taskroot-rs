#![warn(clippy::pedantic, clippy::nursery)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use tauri::Manager;

pub mod domain;
pub mod db;
pub mod screens;

#[tauri::command]
async fn get_tasks(app: tauri::AppHandle) -> Result<Vec<domain::AppTask>, String> {
    let pool = app.try_state::<sqlx::SqlitePool>().ok_or("Database not initialized yet")?;
    db::get_tasks(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_events(app: tauri::AppHandle) -> Result<Vec<domain::AppEvent>, String> {
    let pool = app.try_state::<sqlx::SqlitePool>().ok_or("Database not initialized yet")?;
    db::get_events(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_task(app: tauri::AppHandle, task: domain::AppTask) -> Result<(), String> {
    let pool = app.try_state::<sqlx::SqlitePool>().ok_or("Database not initialized yet")?;
    db::create_task(&pool, task).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_task(app: tauri::AppHandle, task: domain::AppTask) -> Result<(), String> {
    let pool = app.try_state::<sqlx::SqlitePool>().ok_or("Database not initialized yet")?;
    db::update_task(&pool, task).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_task(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let pool = app.try_state::<sqlx::SqlitePool>().ok_or("Database not initialized yet")?;
    db::delete_task(&pool, id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_event(app: tauri::AppHandle, event: domain::AppEvent) -> Result<(), String> {
    let pool = app.try_state::<sqlx::SqlitePool>().ok_or("Database not initialized yet")?;
    db::create_event(&pool, event).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_event(app: tauri::AppHandle, event: domain::AppEvent) -> Result<(), String> {
    let pool = app.try_state::<sqlx::SqlitePool>().ok_or("Database not initialized yet")?;
    db::update_event(&pool, event).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_event(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let pool = app.try_state::<sqlx::SqlitePool>().ok_or("Database not initialized yet")?;
    db::delete_event(&pool, id).await.map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(app_data_dir) = handle.path().app_data_dir() {
                    if std::fs::create_dir_all(&app_data_dir).is_ok() {
                        let db_path = app_data_dir.join("taskroot.db");
                        if let Some(db_path_str) = db_path.to_str() {
                            let db_path_str = format!("sqlite:{db_path_str}");
                            match db::init_db(&db_path_str).await {
                                Ok(pool) => {
                                    handle.manage(pool);
                                }
                                Err(e) => {
                                    eprintln!("Failed to initialize DB: {e}");
                                }
                            }
                        }
                    }
                }
            });
            Ok(())
        })
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_tasks, get_events, 
            create_task, update_task, delete_task,
            create_event, update_event, delete_event,
            screens::plan::get_plan_layout,
            screens::plan::get_filtered_tasks
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| panic!("error while running tauri application: {e}"));
}
