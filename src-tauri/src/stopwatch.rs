use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::Manager;
use ts_rs::TS;

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(
    export,
    export_to = "../../src/lib/bindings/StopwatchState.generated.ts"
)]
pub struct StopwatchState {
    #[ts(type = "number")]
    pub elapsed: u64,
    #[ts(type = "number | null")]
    pub running_since: Option<u64>,
    pub is_break: bool,
    #[ts(type = "number")]
    pub break_elapsed: u64,
    #[ts(type = "number | null")]
    pub break_running_since: Option<u64>,
    #[ts(type = "number | null")]
    pub paused_until: Option<u64>,
}

pub struct StopwatchManager(pub Mutex<StopwatchState>);

pub(crate) fn with_locked_state<T>(
    app: &tauri::AppHandle,
    apply: impl FnOnce(&mut StopwatchState) -> T,
) -> Result<T, AppError> {
    let manager = app
        .try_state::<StopwatchManager>()
        .ok_or_else(|| AppError::Internal("Stopwatch state not initialized yet".to_string()))?;
    let mut guard = manager
        .0
        .lock()
        .map_err(|_| AppError::Internal("Stopwatch state is unavailable".to_string()))?;
    Ok(apply(&mut guard))
}

pub(crate) fn current_epoch_millis() -> Result<u64, AppError> {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| AppError::Internal(format!("System clock error: {e}")))?
        .as_millis();
    u64::try_from(millis).map_err(|_| AppError::Internal("Timestamp too large".to_string()))
}
