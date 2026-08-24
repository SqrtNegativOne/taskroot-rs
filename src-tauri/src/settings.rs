#![allow(clippy::struct_excessive_bools)]

use crate::db;
use crate::error::AppError;
use serde_json::{Map, Value};
use sqlx::SqlitePool;
use std::collections::HashMap;
use tauri::Manager;

// Include the generated AppSettings struct from build.rs
include!(concat!(env!("OUT_DIR"), "/settings_generated.rs"));

#[tauri::command]
#[allow(clippy::must_use_candidate)]
pub fn get_settings_schema() -> Value {
    let schema_json = include_str!(concat!(env!("OUT_DIR"), "/settings_schema.json"));
    serde_json::from_str(schema_json).unwrap_or_default()
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

fn coerce_number(stored: Value) -> Option<Value> {
    match stored {
        n @ Value::Number(_) => Some(n),
        Value::String(s) => s.trim().parse::<i64>().ok().map(Value::from).or_else(|| {
            serde_json::Number::from_f64(s.trim().parse::<f64>().ok()?).map(Value::Number)
        }),
        _ => None,
    }
}

fn coerce_bool(stored: Value) -> Option<Value> {
    match stored {
        b @ Value::Bool(_) => Some(b),
        Value::Number(n) => Some(Value::Bool(n.as_i64().unwrap_or_default() != 0)),
        Value::String(s) => match s.trim().to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Some(Value::Bool(true)),
            "false" | "0" | "no" | "off" => Some(Value::Bool(false)),
            _ => None,
        },
        _ => None,
    }
}

fn coerce_string(stored: Value) -> Value {
    match stored {
        s @ Value::String(_) => s,
        other => Value::String(other.to_string()),
    }
}

fn coerce_to_target_kind(default: &Value, stored: Value) -> Value {
    match default {
        Value::Number(_) => coerce_number(stored).unwrap_or_else(|| default.clone()),
        Value::Bool(_) => coerce_bool(stored).unwrap_or_else(|| default.clone()),
        Value::String(_) => coerce_string(stored),
        _ => stored,
    }
}

fn parse_stored_value(raw: String) -> Value {
    serde_json::from_str::<Value>(&raw).unwrap_or(Value::String(raw))
}

async fn load_stored_settings(pool: &SqlitePool) -> Result<HashMap<String, Value>, AppError> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings")
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(|(k, v)| (k, parse_stored_value(v)))
        .collect())
}

#[tauri::command]
pub async fn get_settings(app: tauri::AppHandle) -> Result<AppSettings, AppError> {
    let pool = app
        .try_state::<SqlitePool>()
        .ok_or_else(|| AppError::NotReady("Database not initialized yet".to_string()))?;

    let stored = load_stored_settings(&pool).await?;
    let mut merged = defaults_object()?;

    for (key, stored_value) in &stored {
        if let Some(default_value) = merged.get(key) {
            let coerced = coerce_to_target_kind(default_value, stored_value.clone());
            merged.insert(key.clone(), coerced);
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
        .ok_or_else(|| AppError::NotReady("Database not initialized yet".to_string()))?;

    let defaults = defaults_object()?;
    let canonical = match defaults.get(&key) {
        Some(default) => coerce_to_target_kind(default, value),
        None => value,
    };

    let value_str = serde_json::to_string(&canonical)
        .map_err(|e| AppError::Internal(format!("Failed to serialize setting: {e}")))?;

    db::set_setting(&pool, &key, &value_str).await?;
    Ok(())
}
