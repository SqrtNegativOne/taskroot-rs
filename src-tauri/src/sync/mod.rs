use crate::commands::session::store_state;
use crate::error::AppError;
use crate::store::{ops, StoreState};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::Duration;
use ts_rs::TS;

#[derive(Clone, Serialize, Deserialize, Default, TS)]
#[ts(export, export_to = "../../src/lib/bindings/SyncState.generated.ts")]
pub struct SyncState {
    pub is_syncing: bool,
    pub error: Option<String>,
    #[ts(type = "string | null")]
    pub next_sync_at: Option<DateTime<Utc>>,
    pub offline: bool,
    #[ts(type = "string | null")]
    pub last_synced: Option<DateTime<Utc>>,
}

pub struct SyncStateManager(pub Mutex<SyncState>);

pub(crate) const SYNC_INTERVAL_SECS: i64 = 5 * 60;
const MIN_SYNC_SECS: u64 = 60;
const MAX_SYNC_SECS: u64 = 24 * 60 * 60;
const TRIGGER_DEBOUNCE_SECS: u64 = 10;

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

/// Start the periodic poller and the debounced edit trigger.
pub fn start_sync_engine(app: AppHandle) {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(100);
    app.manage(SyncTrigger(tx));

    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let interval = poll_interval(&app_clone).await;
            tokio::time::sleep(Duration::from_secs(interval)).await;
            let next_sync = Utc::now().checked_add_signed(chrono::Duration::seconds(
                i64::try_from(interval).unwrap_or(SYNC_INTERVAL_SECS),
            ));
            if let Err(error) = run_tracked_sync(&app_clone, next_sync).await {
                eprintln!("Sync Engine Error: {error}");
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
                    () = tokio::time::sleep(Duration::from_secs(TRIGGER_DEBOUNCE_SECS)) => {
                        break;
                    }
                    opt = rx.recv() => {
                        if opt.is_none() {
                            return;
                        }
                    }
                }
            }
            if let Err(error) = run_tracked_sync(&app, None).await {
                eprintln!("Debounced Sync Error: {error}");
            }
        }
    });
}

async fn poll_interval(app: &AppHandle) -> u64 {
    let Some(state) = app.try_state::<StoreState>() else {
        return u64::try_from(SYNC_INTERVAL_SECS).unwrap_or(300);
    };
    let minutes = state.read(|store| store.settings.sync_interval).await;
    let seconds = u64::try_from(minutes.clamp(1, 1440))
        .unwrap_or(5)
        .saturating_mul(60);
    seconds.clamp(MIN_SYNC_SECS, MAX_SYNC_SECS)
}

#[allow(clippy::unused_async)]
pub(crate) async fn run_tracked_sync(
    app: &AppHandle,
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

    if let Err(error) = sync_all(app).await {
        let message = error.to_string();
        update_state(app, |state| {
            state.is_syncing = false;
            state.error = Some(message.clone());
            state.offline = true;
        });
        let _ = app.emit(crate::events::SYNC_ERROR, message.clone());
        return Err(message);
    }

    update_state(app, |state| {
        state.is_syncing = false;
        state.error = None;
        state.offline = false;
        state.last_synced = Some(Utc::now());
    });
    let _ = app.emit(crate::events::SYNC_FINISHED, ());
    Ok(())
}

fn update_state(app: &AppHandle, update: impl FnOnce(&mut SyncState)) {
    if let Ok(mut guard) = app.state::<SyncStateManager>().0.lock() {
        update(&mut guard);
    }
}

/// Load the account collections and pull them into the store.
///
/// A failed load marks the store offline and falls back to `snapshot.json`
/// (never merging cached and remote state).
///
/// # Errors
///
/// Returns the account/transport/parse error after the offline fallback has been
/// applied; the caller reports it to the UI.
pub async fn sync_all(app: &AppHandle) -> Result<(), AppError> {
    let client = crate::commands::session::client_for(app).await?;
    let state = store_state(app)?;
    let mut store = state.lock().await;

    let dir = crate::app_data_dir(app)?;
    let account = crate::caldav::account::read_account(&dir)?
        .ok_or_else(|| AppError::Auth("No CalDAV account is configured".to_string()))?;
    if store.calendars != account.calendars {
        store.calendars.clone_from(&account.calendars);
        store.resources.clear();
        store.tokens.clear();
    }
    if store.calendars.is_empty() {
        store.mark_synced();
        return store.write_snapshot();
    }

    let result = if store.is_empty() {
        ops::full_load(&mut store, &client).await
    } else {
        ops::pull(&mut store, &client).await
    };

    match result {
        Ok(()) => Ok(()),
        Err(error) => {
            store.offline = true;
            if store.is_empty() {
                if let Err(snapshot_error) = store.load_snapshot() {
                    eprintln!("Failed to load the offline snapshot: {snapshot_error}");
                }
            }
            Err(error)
        }
    }
}
