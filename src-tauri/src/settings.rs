#![allow(clippy::struct_excessive_bools)]

use crate::db;
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sqlx::SqlitePool;
use std::collections::HashMap;
use tauri::Manager;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/AppSettings.generated.ts")]
#[serde(default)]
pub struct AppSettings {
    pub default_calendar_view: String,
    pub day_timeline_start_view: i32,
    pub default_task_duration: i32,
    pub clock_style: String,
    pub allow_stopwatch_without_task: bool,
    pub flowtime_break_divisor: i32,
    pub enable_calendar_sync: bool,
    pub enable_tasks_sync: bool,
    pub sync_interval: i32,
    pub keybinding_launcher: String,
    pub keybinding_open_settings: String,
    pub keybinding_restore_app: String,
    pub tracker_show_border: bool,
    pub tracker_opacity: i32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_calendar_view: "month".to_string(),
            day_timeline_start_view: 300,
            default_task_duration: 0,
            clock_style: "guzey".to_string(),
            allow_stopwatch_without_task: false,
            flowtime_break_divisor: 5,
            enable_calendar_sync: true,
            enable_tasks_sync: true,
            sync_interval: 5,
            keybinding_launcher: "Meta+Shift+Space".to_string(),
            keybinding_open_settings: "Ctrl+,".to_string(),
            keybinding_restore_app: "Ctrl+Alt+R".to_string(),
            tracker_show_border: true,
            tracker_opacity: 80,
        }
    }
}

#[tauri::command]
#[allow(clippy::must_use_candidate)]
#[allow(clippy::too_many_lines)]
pub fn get_settings_schema() -> Value {
    let defaults = AppSettings::default();
    json!({
        "tabs": [
            {
                "id": "plan_screen",
                "label": "Plan screen",
                "sections": [
                    {
                        "name": "Calendar",
                        "settings": [
                            {
                                "id": "default_calendar_view",
                                "label": "Default View",
                                "keywords": ["calendar", "view", "month", "week"],
                                "type": "select",
                                "defaultValue": defaults.default_calendar_view,
                                "options": [
                                    {"value": "month", "label": "Month"},
                                    {"value": "week", "label": "Week"}
                                ]
                            },
                            {
                                "id": "day_timeline_start_view",
                                "label": "Timeline View Start Time",
                                "keywords": ["timeline", "day", "start", "time", "scroll", "view"],
                                "type": "time",
                                "defaultValue": defaults.day_timeline_start_view
                            },
                            {
                                "id": "default_task_duration",
                                "label": "Default Duration",
                                "keywords": ["task", "duration", "estimate", "time"],
                                "type": "select",
                                "defaultValue": defaults.default_task_duration,
                                "options": [
                                    {"value": 0, "label": "Not set"},
                                    {"value": 15, "label": "15m"},
                                    {"value": 30, "label": "30m"},
                                    {"value": 45, "label": "45m"}
                                ]
                            }
                        ]
                    }
                ]
            },
            {
                "id": "do_screen",
                "label": "Do screen",
                "sections": [
                    {
                        "name": "Stopwatch",
                        "settings": [
                            {
                                "id": "clock_style",
                                "label": "Clock Style",
                                "keywords": ["stopwatch", "timer", "guzey", "counter", "flowtime"],
                                "type": "select",
                                "defaultValue": defaults.clock_style,
                                "options": [
                                    {"value": "counter", "label": "Counter"},
                                    {"value": "flowtime", "label": "Flowtime"},
                                    {"value": "guzey", "label": "Guzey"}
                                ]
                            },
                            {
                                "id": "allow_stopwatch_without_task",
                                "label": "Allow stopwatch use without selecting task",
                                "keywords": ["stopwatch", "task", "requirement", "allow"],
                                "type": "checkbox",
                                "defaultValue": defaults.allow_stopwatch_without_task
                            },
                            {
                                "id": "flowtime_break_divisor",
                                "label": "Flowtime Break Divisor",
                                "description": "How much break time you earn (e.g. 5 means 1 min break for every 5 mins of work).",
                                "keywords": ["flowtime", "break", "divisor", "rest"],
                                "type": "number",
                                "defaultValue": defaults.flowtime_break_divisor,
                                "min": 1
                            }
                        ]
                    }
                ]
            },
            {
                "id": "sync",
                "label": "Sync and Backup",
                "sections": [
                    {
                        "name": "Sync & Integrations",
                        "settings": [
                            {
                                "id": "enable_calendar_sync",
                                "label": "Enable Bidirectional Google Calendar Sync",
                                "description": "Self explanatory.",
                                "keywords": ["google", "calendar", "sync", "events"],
                                "type": "checkbox",
                                "defaultValue": defaults.enable_calendar_sync
                            },
                            {
                                "id": "enable_tasks_sync",
                                "label": "Enable Bidirectional Google Tasks Sync",
                                "description": "Self explanatory.",
                                "keywords": ["google", "tasks", "sync", "todos"],
                                "type": "checkbox",
                                "defaultValue": defaults.enable_tasks_sync
                            },
                            {
                                "id": "sync_interval",
                                "label": "Sync Interval (minutes)",
                                "keywords": ["sync", "interval", "poll", "time"],
                                "type": "number",
                                "defaultValue": defaults.sync_interval,
                                "min": 1
                            },
                            {
                                "id": "logout",
                                "label": "Sign out",
                                "description": "Sign out of your Google account.",
                                "keywords": ["logout", "signout", "google", "account"],
                                "type": "custom"
                            }
                        ]
                    },
                    {
                        "name": "Danger Zone",
                        "settings": [
                            {
                                "id": "clear_all_data",
                                "label": "Clear All Data",
                                "description": "Permanently delete all your tasks, settings, logs, and other data from both this device and the cloud. This cannot be undone.",
                                "keywords": ["delete", "clear", "wipe", "reset", "factory", "all"],
                                "type": "custom",
                                "danger": true
                            }
                        ]
                    }
                ]
            },
            {
                "id": "keybindings",
                "label": "Keybindings",
                "sections": [
                    {
                        "name": "Keybindings",
                        "settings": [
                            {
                                "id": "keybinding_launcher",
                                "label": "Open Launcher",
                                "keywords": ["keyboard", "shortcut", "launcher", "open"],
                                "type": "keybinding",
                                "defaultValue": defaults.keybinding_launcher
                            },
                            {
                                "id": "keybinding_open_settings",
                                "label": "Open Settings",
                                "keywords": ["keyboard", "shortcut", "settings", "open"],
                                "type": "keybinding",
                                "defaultValue": defaults.keybinding_open_settings
                            },
                            {
                                "id": "keybinding_restore_app",
                                "label": "Restore App",
                                "keywords": ["keyboard", "shortcut", "restore", "maximize", "mini tracker", "minitracker"],
                                "type": "keybinding",
                                "defaultValue": defaults.keybinding_restore_app
                            }
                        ]
                    }
                ]
            },
            {
                "id": "tracker_window",
                "label": "Tracker window",
                "sections": [
                    {
                        "name": "Appearance",
                        "settings": [
                            {
                                "id": "tracker_show_border",
                                "label": "Show Window Border",
                                "keywords": ["tracker", "border", "show", "outline"],
                                "type": "checkbox",
                                "defaultValue": defaults.tracker_show_border
                            },
                            {
                                "id": "tracker_opacity",
                                "label": "Base Opacity (%)",
                                "description": "The baseline opacity of the mini tracker window (0 to 100).",
                                "keywords": ["tracker", "opacity", "transparent", "window"],
                                "type": "number",
                                "defaultValue": defaults.tracker_opacity,
                                "min": 0,
                                "max": 100
                            }
                        ]
                    }
                ]
            }
        ]
    })
}

fn defaults_object() -> Result<Map<String, Value>, AppError> {
    match serde_json::to_value(AppSettings::default()) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => Err(AppError::Internal(
            "Settings defaults are not a JSON object".to_string(),
        )),
        Err(e) => Err(AppError::Internal(format!(
            "Failed to serialize settings defaults: {e}"
        ))),
    }
}

async fn load_stored_settings(pool: &SqlitePool) -> Result<HashMap<String, Value>, AppError> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings")
        .fetch_all(pool)
        .await?;

    let map: HashMap<String, Value> = rows
        .into_iter()
        .map(|(k, v)| {
            let parsed = serde_json::from_str(&v).unwrap_or(Value::String(v));
            (k, parsed)
        })
        .collect();

    Ok(map)
}

#[tauri::command]
pub async fn get_settings(app: tauri::AppHandle) -> Result<AppSettings, AppError> {
    let pool = app
        .try_state::<SqlitePool>()
        .ok_or_else(|| AppError::Internal("Database not initialized yet".to_string()))?;

    let stored = load_stored_settings(&pool).await?;
    let mut merged = defaults_object()?;

    for (key, stored_value) in stored {
        if let Some(default_value) = merged.get(&key) {
            let types_match = matches!(
                (default_value, &stored_value),
                (Value::Bool(_), Value::Bool(_))
                    | (Value::Number(_), Value::Number(_))
                    | (Value::String(_), Value::String(_))
            );

            if types_match {
                merged.insert(key, stored_value);
            }
        }
    }

    serde_json::from_value(Value::Object(merged))
        .map_err(|e| AppError::Internal(format!("Stored settings failed validation: {e}")))
}

#[tauri::command]
pub async fn update_setting(
    app: tauri::AppHandle,
    key: String,
    value: Value,
) -> Result<(), AppError> {
    let pool = app
        .try_state::<SqlitePool>()
        .ok_or_else(|| AppError::Internal("Database not initialized yet".to_string()))?;

    let value_str = serde_json::to_string(&value)
        .map_err(|e| AppError::Internal(format!("Failed to serialize setting: {e}")))?;

    db::set_setting(&pool, &key, &value_str).await?;
    Ok(())
}
