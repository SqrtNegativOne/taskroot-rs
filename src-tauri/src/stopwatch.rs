#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_possible_truncation)]

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{Manager, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopwatchState {
    pub elapsed: u64,
    pub running_since: Option<u64>,
    pub is_break: bool,
    pub break_allowed_ms: u64,
    pub break_started_at: Option<u64>,
    pub break_sound_played: bool,
}

impl Default for StopwatchState {
    fn default() -> Self {
        Self {
            elapsed: 0,
            running_since: None,
            is_break: false,
            break_allowed_ms: 5 * 60 * 1000,
            break_started_at: None,
            break_sound_played: false,
        }
    }
}

pub struct StopwatchManager(pub Mutex<StopwatchState>);

#[tauri::command]
pub fn get_stopwatch_state(app: tauri::AppHandle) -> Result<StopwatchState, String> {
    let state = app.state::<StopwatchManager>();
    let s = state.0.lock().map_err(|e| e.to_string())?.clone();
    Ok(s)
}

#[tauri::command]
pub fn toggle_stopwatch(app: tauri::AppHandle) -> Result<StopwatchState, String> {
    let state = app.state::<StopwatchManager>();
    let updated = {
        let mut s = state.0.lock().map_err(|e| e.to_string())?;
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis() as u64;

        if let Some(since) = s.running_since {
            s.elapsed += now.saturating_sub(since);
            s.running_since = None;
        } else {
            s.running_since = Some(now);
        }
        
        s.clone()
    };
    let _ = app.emit("stopwatch-updated", &updated);
    Ok(updated)
}

#[tauri::command]
pub fn reset_stopwatch(app: tauri::AppHandle) -> Result<StopwatchState, String> {
    let state = app.state::<StopwatchManager>();
    let updated = {
        let mut s = state.0.lock().map_err(|e| e.to_string())?;
        
        s.elapsed = 0;
        s.running_since = None;
        
        s.clone()
    };
    let _ = app.emit("stopwatch-updated", &updated);
    Ok(updated)
}

#[tauri::command]
pub fn set_stopwatch_state(app: tauri::AppHandle, new_state: StopwatchState) -> Result<(), String> {
    let state = app.state::<StopwatchManager>();
    {
        let mut s = state.0.lock().map_err(|e| e.to_string())?;
        *s = new_state.clone();
    }
    let _ = app.emit("stopwatch-updated", &new_state);
    Ok(())
}
