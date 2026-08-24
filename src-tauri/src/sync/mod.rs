pub mod push;
pub mod queue;
mod queue_store;
pub mod types;

use crate::auth;
use crate::error::AppError;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::{interval, Duration};
use ts_rs::TS;

#[derive(Clone, Serialize, Deserialize, Default, TS)]
#[ts(export, export_to = "../../src/lib/bindings/SyncState.generated.ts")]
pub struct SyncState {
    pub is_syncing: bool,
    pub error: Option<String>,
    #[ts(type = "string | null")]
    pub next_sync_at: Option<DateTime<Utc>>,
}

pub struct SyncStateManager(pub Mutex<SyncState>);

pub(crate) const SYNC_INTERVAL_SECS: i64 = 5 * 60;

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn get_sync_state(app: tauri::AppHandle) -> Result<SyncState, AppError> {
    let state = app
        .try_state::<SyncStateManager>()
        .ok_or_else(|| AppError::NotReady("Sync state not initialized yet".to_string()))?;
    Ok(state
        .0
        .lock()
        .map_or_else(|_| SyncState::default(), |guard| guard.clone()))
}

pub fn start_sync_engine(app: AppHandle, pool: SqlitePool) {
    let pool = Arc::new(pool);
    tauri::async_runtime::spawn(async move {
        let mut interval = interval(Duration::from_secs(SYNC_INTERVAL_SECS as u64));

        loop {
            interval.tick().await;

            let next_sync = Utc::now() + chrono::Duration::seconds(SYNC_INTERVAL_SECS);

            if let Err(e) = run_tracked_sync(&app, &pool, next_sync).await {
                eprintln!("Sync Engine Error: {e}");
            }
        }
    });
}

pub(crate) async fn run_tracked_sync(
    app: &AppHandle,
    pool: &SqlitePool,
    next_sync_at: DateTime<Utc>,
) -> Result<(), String> {
    if let Ok(mut guard) = app.state::<SyncStateManager>().0.lock() {
        guard.is_syncing = true;
        guard.error = None;
        guard.next_sync_at = Some(next_sync_at);
    }
    let _ = app.emit(crate::events::SYNC_STARTED, ());

    if let Err(e) = sync_with_google(pool).await {
        let err_str = e.to_string();
        if let Ok(mut guard) = app.state::<SyncStateManager>().0.lock() {
            guard.is_syncing = false;
            guard.error = Some(err_str.clone());
        }
        let _ = app.emit(crate::events::SYNC_ERROR, err_str.clone());
        return Err(err_str);
    }

    if let Ok(mut guard) = app.state::<SyncStateManager>().0.lock() {
        guard.is_syncing = false;
        guard.error = None;
    }
    let _ = app.emit(crate::events::SYNC_FINISHED, ());
    Ok(())
}

pub async fn sync_with_google(pool: &SqlitePool) -> Result<()> {
    let Ok(access_token) = auth::get_valid_access_token(pool).await else {
        return Ok(());
    };

    // --- PUSH: Publish local queued items ---
    let queue = crate::sync::queue::SyncQueue::new(Arc::new(pool.clone()));
    while let Ok(Some((queue_id, item))) = queue.peek().await {
        let mut success = true;
        match item.item {
            crate::sync::types::SyncItemData::Task(mut task) => {
                if item.action == crate::sync::types::SyncAction::Delete {
                    if let Some(remote_id) = &task.remote_id {
                        if crate::apis::google_tasks::delete(remote_id, &access_token)
                            .await
                            .is_err()
                        {
                            success = false;
                        }
                    }
                } else {
                    match crate::apis::google_tasks::publish(&task, &access_token).await {
                        Ok(remote_id) => {
                            task.remote_id = Some(remote_id);
                            task.dirty = Some(false);
                            let _ = crate::db::upsert_task(pool, task).await;
                        }
                        Err(_) => {
                            success = false;
                        }
                    }
                }
            }
            crate::sync::types::SyncItemData::Event(mut event) => {
                if item.action == crate::sync::types::SyncAction::Delete {
                    if let Some(remote_id) = &event.remote_id {
                        if crate::apis::google_calendar::delete(remote_id, &access_token)
                            .await
                            .is_err()
                        {
                            success = false;
                        }
                    }
                } else {
                    match crate::apis::google_calendar::publish(&event, &access_token).await {
                        Ok(remote_id) => {
                            event.remote_id = Some(remote_id);
                            event.dirty = Some(false);
                            let _ = crate::db::upsert_event(pool, event).await;
                        }
                        Err(_) => {
                            success = false;
                        }
                    }
                }
            }
        }

        if success {
            let _ = queue.remove(queue_id).await;
        } else {
            // Stop pushing if we encounter an error, to avoid out-of-order execution and infinite loops
            break;
        }
    }

    // --- PULL: Fetch remote items ---

    if let Err(e) = crate::apis::google_calendar::sync(pool, &access_token).await {
        eprintln!("Google Calendar Sync Error: {e}");
    }

    if let Err(e) = crate::apis::google_tasks::sync(pool, &access_token).await {
        eprintln!("Google Tasks Sync Error: {e}");
    }

    Ok(())
}
