pub mod queue;
pub mod types;

use crate::auth;
use anyhow::Result;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tauri::{AppHandle, Emitter};

pub fn start_sync_engine(app: AppHandle, pool: SqlitePool) {
    let pool = Arc::new(pool);
    tauri::async_runtime::spawn(async move {
        let mut interval = interval(Duration::from_secs(5 * 60)); // Every 5 minutes
        
        loop {
            interval.tick().await;
            println!("Sync Engine: Checking for updates...");
            
            let _ = app.emit("sync-started", ());

            if let Err(e) = sync_with_google(&pool).await {
                eprintln!("Sync Engine Error: {e}");
                let _ = app.emit("sync-error", e.to_string());
            } else {
                let _ = app.emit("sync-finished", ());
            }
        }
    });
}

async fn sync_with_google(pool: &SqlitePool) -> Result<()> {
    let Ok(access_token) = auth::get_valid_access_token(pool).await else {
        return Ok(());
    };

    // --- PUSH: Publish local queued items ---
    let queue = crate::sync::queue::SyncQueue::new(Arc::new(pool.clone()));
    while let Ok(Some((_queue_id, item))) = queue.shift().await {
        match item.item {
            crate::sync::types::SyncItemData::Task(mut task) => {
                if item.action == crate::sync::types::SyncAction::Delete {
                    if let Some(remote_id) = &task.remote_id {
                        let _ = crate::apis::google_tasks::delete(remote_id, &access_token).await;
                    }
                    continue;
                }
                if let Ok(remote_id) = crate::apis::google_tasks::publish(&task, &access_token).await {
                    task.remote_id = Some(remote_id);
                    task.dirty = Some(false);
                    let _ = crate::db::upsert_task(pool, task).await;
                }
            },
            crate::sync::types::SyncItemData::Event(mut event) => {
                if item.action == crate::sync::types::SyncAction::Delete {
                    if let Some(remote_id) = &event.remote_id {
                        let _ = crate::apis::google_calendar::delete(remote_id, &access_token).await;
                    }
                    continue;
                }
                if let Ok(remote_id) = crate::apis::google_calendar::publish(&event, &access_token).await {
                    event.remote_id = Some(remote_id);
                    event.dirty = Some(false);
                    let _ = crate::db::upsert_event(pool, event).await;
                }
            }
        }
    }

    // --- PULL: Fetch remote items ---

    if let Err(e) = crate::apis::google_calendar::sync(pool, &access_token).await {
        eprintln!("Google Calendar Sync Error: {}", e);
    }

    if let Err(e) = crate::apis::google_tasks::sync(pool, &access_token).await {
        eprintln!("Google Tasks Sync Error: {}", e);
    }

    Ok(())
}
