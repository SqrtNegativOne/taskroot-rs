//! Whole-mirror maintenance statements.
//!
//! These back the `wipe_local_data` command (the settings screen's "clear all
//! data" button and the dev inspector): the local mirror of remote data is
//! dropped so the next sync can refetch it.

use sqlx::SqlitePool;

/// Delete every locally mirrored task and event row.
///
/// # Errors
///
/// Returns an error if either delete fails.
pub async fn wipe_local_mirror(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM tasks").execute(pool).await?;
    sqlx::query("DELETE FROM events").execute(pool).await?;
    Ok(())
}
