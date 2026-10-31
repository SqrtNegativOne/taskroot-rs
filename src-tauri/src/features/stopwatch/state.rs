//! Stopwatch state and clock helpers. No Tauri dependency.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;

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

pub fn current_epoch_millis() -> Result<u64, AppError> {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| AppError::Internal(format!("System clock error: {e}")))?
        .as_millis();
    u64::try_from(millis).map_err(|_| AppError::Internal("Timestamp too large".to_string()))
}
