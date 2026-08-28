pub mod push;
pub mod queue;
mod queue_store;
pub mod types;

use crate::auth;
use crate::error::AppError;
use color_eyre::Result;
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

pub struct SyncTrigger(pub tokio::sync::mpsc::Sender<()>);

pub fn trigger_sync(app: &AppHandle) {
    if let Some(trigger) = app.try_state::<SyncTrigger>() {
        let _ = trigger.0.try_send(());
    }
}

pub fn start_sync_engine(app: AppHandle, pool: SqlitePool) {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(100);
    app.manage(SyncTrigger(tx));

    let pool = Arc::new(pool);
    let app_clone = app.clone();
    let pool_clone = pool.clone();

    tauri::async_runtime::spawn(async move {
        let mut interval = interval(Duration::from_secs(u64::try_from(SYNC_INTERVAL_SECS).unwrap_or(300)));

        loop {
            interval.tick().await;

            let next_sync = Utc::now().checked_add_signed(chrono::Duration::seconds(SYNC_INTERVAL_SECS)).unwrap_or_else(Utc::now);

            if let Err(e) = run_tracked_sync(&app_clone, &pool_clone, Some(next_sync)).await {
                eprintln!("Sync Engine Error: {e}");
            }
        }
    });

    tauri::async_runtime::spawn(async move {
        loop {
            if rx.recv().await.is_none() {
                break;
            }

            loop {
                tokio::select! {
                    () = tokio::time::sleep(Duration::from_secs(10)) => {
                        break;
                    }
                    opt = rx.recv() => {
                        if opt.is_none() {
                            return;
                        }
                    }
                }
            }

            if let Err(e) = run_tracked_sync(&app, &pool, None).await {
                eprintln!("Debounced Sync Error: {e}");
            }
        }
    });
}

pub(crate) async fn run_tracked_sync(
    app: &AppHandle,
    pool: &SqlitePool,
    next_sync_at: Option<DateTime<Utc>>,
) -> Result<(), String> {
    if let Ok(mut guard) = app.state::<SyncStateManager>().0.lock() {
        guard.is_syncing = true;
        guard.error = None;
        if let Some(time) = next_sync_at {
            guard.next_sync_at = Some(time);
        }
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
                            task.remote_id = Some(crate::domain::RemoteId(remote_id));
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
                        if crate::apis::google_calendar::delete(remote_id, event.remote_collection_id.as_deref().map(String::as_str), &access_token)
                            .await
                            .is_err()
                        {
                            success = false;
                        }
                    }
                } else {
                    match crate::apis::google_calendar::publish(&event, &access_token).await {
                        Ok(remote_id) => {
                            event.remote_id = Some(crate::domain::RemoteId(remote_id));
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

