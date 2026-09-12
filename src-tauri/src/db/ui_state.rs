use sqlx::SqlitePool;

/// Read one per-component UI-state row. Returns `None` for an unknown key.
///
/// # Errors
///
/// Returns an error if the query fails.
pub async fn get_ui_state(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM ui_state WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.0))
}

/// Upsert one per-component UI-state row.
///
/// # Errors
///
/// Returns an error if the write fails.
pub async fn set_ui_state(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ui_state (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}
