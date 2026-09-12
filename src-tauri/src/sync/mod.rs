pub mod push;
pub mod queue;
mod drain;
mod queue_store;
pub mod types;

use crate::auth;
use crate::error::AppError;
use chrono::{DateTime, Utc};
use color_eyre::Result;
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

/// Drop every queued offline sync action.
///
/// The `clear_sync_queue` command body; the queue's SQL stays in [`queue_store`].
///
/// # Errors
///
/// Returns an error if the delete fails.
pub(crate) async fn clear_queue(pool: &SqlitePool) -> Result<(), AppError> {
    queue_store::clear(pool).await?;
    Ok(())
}

/// The queued payloads, oldest first, as the raw JSON the queue stored.
///
/// The `get_sync_queue` command body (the dev inspector renders these as-is).
///
/// # Errors
///
/// Returns an error if the queue cannot be read.
pub(crate) async fn queue_payloads(
    pool: &SqlitePool,
) -> Result<Vec<serde_json::Value>, AppError> {
    let payloads = queue_store::fetch_payloads(pool).await?;
    Ok(payloads
        .iter()
        .filter_map(|payload| serde_json::from_str(payload).ok())
        .collect())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn get_sync_state(app: tauri::AppHandle) -> Result<SyncState, AppError> {
    let state = app
        .try_state::<SyncStateManager>()
        .ok_or_else(|| AppError::Internal("Sync state not initialized yet".to_string()))?;
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
        let mut interval = interval(Duration::from_secs(
            u64::try_from(SYNC_INTERVAL_SECS).unwrap_or(300),
        ));

        loop {
            interval.tick().await;

            let next_sync = Utc::now()
                .checked_add_signed(chrono::Duration::seconds(SYNC_INTERVAL_SECS))
                .unwrap_or_else(Utc::now);

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
    let push_error = drain::flush_queue(pool, &access_token).await;

    // --- PULL: Fetch remote items ---

    if let Err(e) = crate::apis::google_calendar::sync(pool, &access_token).await {
        eprintln!("Google Calendar Sync Error: {e}");
    }

    if let Err(e) = crate::apis::google_tasks::sync(pool, &access_token).await {
        eprintln!("Google Tasks Sync Error: {e}");
    }

    push_error.map_or_else(|| Ok(()), |message| Err(color_eyre::eyre::eyre!("{message}")))
}
