//! Reading, validating and merging persisted setting values.
//!
//! The merge rule lives here in one place: a stored value is merged over the
//! defaults only when its key names an [`AppSettings`] field and the value
//! deserializes into that field's serde type. Everything else keeps the default.
//!
//! The pool-level bodies of the settings commands live here too
//! ([`read_settings`], [`write_setting`], [`read_ui_state`], [`write_ui_state`])
//! so they can be driven from tests without an `AppHandle`; the command functions
//! in the parent module only resolve the pool and delegate.

use crate::db;
use crate::error::AppError;
use serde_json::{Map, Value};
use sqlx::SqlitePool;
use std::collections::HashMap;

use super::AppSettings;

/// Decode one stored `TEXT` column into JSON.
///
/// Values are JSON by construction; a bare string (an older row or a hand-edited
/// database) falls back to `Value::String` so one malformed row cannot break a
/// whole read path.
fn decode_stored_value(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
}

/// Decode an optional stored column. Missing keys become `Null` so the frontend
/// can distinguish "never saved" from a stored `null`.
pub(super) fn parse_setting_value(raw: Option<&str>) -> Value {
    raw.map_or(Value::Null, decode_stored_value)
}

/// Encode a value for storage in a `TEXT` column.
///
/// Shared by the settings and UI-state write paths so their serialization
/// contract cannot drift.
pub(super) fn encode_stored_value(value: &Value) -> Result<String, AppError> {
    serde_json::to_string(value)
        .map_err(|e| AppError::Internal(format!("Failed to serialize value: {e}")))
}

pub(super) async fn load_stored_settings(
    pool: &SqlitePool,
) -> Result<HashMap<String, Value>, AppError> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings")
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|(key, value)| (key, decode_stored_value(&value)))
        .collect())
}

/// The JSON object of default setting values.
///
/// `AppSettings` is a plain struct of JSON-safe primitives, so serialization
/// cannot fail; the fallback only satisfies the type.
fn defaults_object() -> Map<String, Value> {
    serde_json::to_value(AppSettings::default())
        .ok()
        .and_then(|value| match value {
            Value::Object(map) => Some(map),
            _ => None,
        })
        .unwrap_or_default()
}

/// The `get_settings` command body: merge the stored rows over the defaults.
///
/// # Errors
///
/// Returns an error if the stored rows cannot be read.
pub(super) async fn read_settings(pool: &SqlitePool) -> Result<AppSettings, AppError> {
    let stored = load_stored_settings(pool).await?;
    Ok(apply_stored_settings(stored))
}

/// The `update_setting` command body: validate the value for a known key,
/// encode it and persist it.
///
/// # Errors
///
/// Returns an error if a known key rejects the value, or the write fails.
pub(super) async fn write_setting(
    pool: &SqlitePool,
    key: &str,
    value: &Value,
) -> Result<(), AppError> {
    ensure_valid_setting_value(key, value)?;
    db::set_setting(pool, key, &encode_stored_value(value)?).await?;
    Ok(())
}

/// The `get_ui_state` command body: decode one per-component UI-state row.
///
/// # Errors
///
/// Returns an error if the row cannot be read.
pub(super) async fn read_ui_state(pool: &SqlitePool, key: &str) -> Result<Value, AppError> {
    let raw = db::get_ui_state(pool, key).await?;
    Ok(parse_setting_value(raw.as_deref()))
}

/// The `set_ui_state` command body: encode and persist one UI-state row.
///
/// # Errors
///
/// Returns an error if the value cannot be encoded or the write fails.
pub(super) async fn write_ui_state(
    pool: &SqlitePool,
    key: &str,
    value: &Value,
) -> Result<(), AppError> {
    db::set_ui_state(pool, key, &encode_stored_value(value)?).await?;
    Ok(())
}

/// The merge rule: `key` names an `AppSettings` field and `value` deserializes
/// into that field's serde type.
///
/// The probe is a one-field object; the struct's container-level
/// `#[serde(default)]` fills every other field from `AppSettings::default()`,
/// so only `key` is exercised. The struct itself therefore defines which values
/// are acceptable and no hand-maintained list of JSON variants can drift out of
/// sync. Unknown keys are rejected here (serde would otherwise ignore them).
fn accepts_field_value(defaults: &Map<String, Value>, key: &str, value: &Value) -> bool {
    if !defaults.contains_key(key) {
        return false;
    }
    let mut probe = Map::new();
    probe.insert(key.to_string(), value.clone());
    serde_json::from_value::<AppSettings>(Value::Object(probe)).is_ok()
}

/// Reject a known `AppSettings` key whose value cannot be deserialized into its
/// field, instead of storing a value [`super::get_settings`] will silently ignore.
///
/// Unknown keys stay allowed so the command remains a generic key/value setter.
pub(super) fn ensure_valid_setting_value(key: &str, value: &Value) -> Result<(), AppError> {
    let defaults = defaults_object();
    if defaults.contains_key(key) && !accepts_field_value(&defaults, key, value) {
        return Err(AppError::InvalidInput(format!(
            "Setting '{key}' does not accept the provided value"
        )));
    }
    Ok(())
}

/// Merge stored rows over the defaults: unknown keys are ignored, and a known key
/// whose stored value cannot be deserialized into its field keeps the default.
pub(super) fn apply_stored_settings(stored: HashMap<String, Value>) -> AppSettings {
    let mut merged = defaults_object();

    for (key, value) in stored {
        if accepts_field_value(&merged, &key, &value) {
            merged.insert(key, value);
        }
    }

    // Every merged value already passed the per-field serde probe, so the struct
    // is guaranteed to deserialize; the fallback is unreachable.
    serde_json::from_value(Value::Object(merged)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn stored(entries: &[(&str, Value)]) -> HashMap<String, Value> {
        entries
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect()
    }

    #[test]
    fn parse_setting_value_returns_null_for_missing_key() {
        assert_eq!(parse_setting_value(None), Value::Null);
    }

    #[test]
    fn parse_setting_value_decodes_stored_json() {
        let raw = Some(r#"[{"column":"status","value":["done"]}]"#);

        let parsed = parse_setting_value(raw);

        let first = parsed.as_array().expect("array").first().expect("item");
        assert_eq!(first["column"], Value::String("status".to_string()));
    }

    #[test]
    fn parse_setting_value_falls_back_to_raw_string() {
        assert_eq!(
            parse_setting_value(Some("not json")),
            Value::String("not json".to_string())
        );
    }

    #[test]
    fn apply_stored_settings_keeps_defaults_for_missing_keys() {
        let settings = apply_stored_settings(HashMap::new());

        assert_eq!(settings.clock_style, AppSettings::default().clock_style);
    }

    #[test]
    fn apply_stored_settings_overrides_a_correctly_typed_value() {
        let settings = apply_stored_settings(stored(&[(
            "clock_style",
            Value::String("counter".to_string()),
        )]));

        assert_eq!(settings.clock_style, "counter");
    }

    #[test]
    fn apply_stored_settings_ignores_a_wrongly_typed_value() {
        let settings = apply_stored_settings(stored(&[("clock_style", serde_json::json!(42))]));

        assert_eq!(settings.clock_style, AppSettings::default().clock_style);
    }

    #[test]
    fn apply_stored_settings_ignores_structured_values_and_unknown_keys() {
        let settings = apply_stored_settings(stored(&[
            ("clock_style", serde_json::json!(["guzey"])),
            ("sync_interval", Value::Null),
            ("google_access_token", Value::String("token".to_string())),
        ]));

        assert_eq!(
            serde_json::to_value(&settings).expect("settings serialize"),
            serde_json::to_value(AppSettings::default()).expect("defaults serialize")
        );
    }

    #[test]
    fn apply_stored_settings_ignores_a_non_integer_number_without_failing_the_read() {
        let settings = apply_stored_settings(stored(&[
            ("sync_interval", serde_json::json!(1.5)),
            ("clock_style", Value::String("counter".to_string())),
        ]));

        assert_eq!(settings.sync_interval, AppSettings::default().sync_interval);
        assert_eq!(settings.clock_style, "counter");
    }

    #[test]
    fn apply_stored_settings_keeps_booleans_and_numbers() {
        let settings = apply_stored_settings(stored(&[
            ("enable_calendar_sync", Value::Bool(false)),
            ("sync_interval", serde_json::json!(15)),
        ]));

        assert!(!settings.enable_calendar_sync);
        assert_eq!(settings.sync_interval, 15);
    }

    #[test]
    fn ensure_valid_setting_value_rejects_a_wrongly_typed_known_setting() {
        let error = ensure_valid_setting_value("sync_interval", &serde_json::json!("15"))
            .expect_err("a string must be rejected for an i32 setting");

        assert_eq!(error.code(), "invalid-input");
    }

    #[test]
    fn ensure_valid_setting_value_accepts_typed_values_and_unknown_keys() {
        ensure_valid_setting_value("sync_interval", &serde_json::json!(15))
            .expect("a number is valid");
        ensure_valid_setting_value("not_a_setting", &serde_json::json!([1, 2]))
            .expect("unknown keys stay generic");
    }

    #[test]
    fn encode_stored_value_round_trips_json() {
        let encoded = encode_stored_value(&serde_json::json!({ "a": [1, 2] })).expect("encode");
        assert_eq!(
            decode_stored_value(&encoded),
            serde_json::json!({ "a": [1, 2] })
        );
    }
}
