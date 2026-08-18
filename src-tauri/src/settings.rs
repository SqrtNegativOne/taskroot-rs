use crate::db;
use serde_json::Value;
use sqlx::SqlitePool;

// Include the generated AppSettings struct from build.rs
include!(concat!(env!("OUT_DIR"), "/settings_generated.rs"));

#[tauri::command]
pub fn get_settings_schema() -> Value {
    let schema_json = include_str!(concat!(env!("OUT_DIR"), "/settings_schema.json"));
    serde_json::from_str(schema_json).unwrap_or_default()
}

#[tauri::command]
pub async fn get_settings(app: tauri::AppHandle) -> Result<AppSettings, String> {
    use tauri::Manager;
    let pool = app
        .try_state::<SqlitePool>()
        .ok_or("Database not initialized yet")?;
        
    let mut settings = AppSettings::default();
    
    // Parse it to a Map to easily insert generic DB values over defaults
    let mut map = match serde_json::to_value(&settings) {
        Ok(Value::Object(m)) => m,
        _ => return Ok(settings),
    };
    
    // Override with DB values
    let db_rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings")
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;
        
    for (key, val_str) in db_rows {
        if let Ok(parsed_val) = serde_json::from_str::<Value>(&val_str) {
            map.insert(key, parsed_val);
        } else {
            // Fallback for legacy plain text values without quotes
            map.insert(key, Value::String(val_str));
        }
    }
    
    // Deserialize back into the strongly typed struct
    if let Ok(typed_settings) = serde_json::from_value(Value::Object(map)) {
        settings = typed_settings;
    }
    
    Ok(settings)
}

#[tauri::command]
pub async fn update_setting(
    app: tauri::AppHandle,
    key: String,
    value: Value,
) -> Result<(), String> {
    use tauri::Manager;
    let pool = app
        .try_state::<SqlitePool>()
        .ok_or("Database not initialized yet")?;
        
    // Ensure all values are JSON strings
    let value_str = serde_json::to_string(&value).unwrap_or_default();

    db::set_setting(&pool, &key, &value_str)
        .await
        .map_err(|e| e.to_string())
}
