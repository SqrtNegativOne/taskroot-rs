//! Schema creation and additive migrations for the local SQLite database.

use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;

/// Add a column to a pre-existing table when it is missing.
///
/// Table/column names are literals so the SQL stays `'static` (sqlx 0.9
/// rejects dynamically built query strings).
macro_rules! ensure_column {
    ($pool:expr, $table:literal, $column:literal, $alter:literal) => {{
        let existing: Vec<(String,)> = sqlx::query_as::<_, (String,)>(concat!(
            "SELECT name FROM pragma_table_info('",
            $table,
            "')"
        ))
        .fetch_all(&$pool)
        .await?;
        if !existing.iter().any(|(name,)| name == $column) {
            sqlx::query($alter).execute(&$pool).await?;
        }
    }};
}

/// The full local schema. Every statement is `IF NOT EXISTS`, so applying it to an
/// existing database is a no-op; additive column migrations handle tables that
/// predate a column.
const SCHEMA_SQL: &str = "CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            status TEXT,
            priority INTEGER,
            tags TEXT,
            subtasks TEXT,
            parent_task TEXT,
            dependencies TEXT,
            est INTEGER,
            added TEXT,
            canvas_x REAL,
            canvas_y REAL,
            on_canvas BOOLEAN,
            remote_id TEXT,
            notes TEXT,
            tabs TEXT,
            due TEXT,
            updated_at TEXT,
            etag TEXT,
            dirty BOOLEAN DEFAULT 0,
            task_list_id TEXT
        );

        CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            remote_id TEXT,
            remote_collection_id TEXT,
            task_id TEXT,
            title TEXT NOT NULL,
            description TEXT,
            start_time TEXT NOT NULL,
            end_time TEXT NOT NULL,
            rrule TEXT,
            exdates TEXT,
            recurring_event_id TEXT,
            original_start_time TEXT,
            status TEXT,
            updated_at TEXT,
            color TEXT,
            etag TEXT,
            dirty BOOLEAN DEFAULT 0,
            is_all_day BOOLEAN DEFAULT 0,
            timezone TEXT
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS ui_state (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sync_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            item_type TEXT NOT NULL,
            item_id TEXT NOT NULL,
            action TEXT NOT NULL,
            payload TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS tags (
            id TEXT PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            color TEXT
        );

        CREATE TABLE IF NOT EXISTS task_tags (
            task_id TEXT NOT NULL,
            tag_id TEXT NOT NULL,
            PRIMARY KEY (task_id, tag_id),
            FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE,
            FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS calendars (
            id TEXT PRIMARY KEY,
            summary TEXT NOT NULL,
            color TEXT,
            is_primary BOOLEAN DEFAULT 0,
            access_role TEXT,
            sync_token TEXT
        );";

/// # Errors
///
/// Returns an error if connecting or running schema creation fails.
pub async fn init_db(db_path: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(db_path)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePool::connect_with(options).await?;

    sqlx::query(SCHEMA_SQL).execute(&pool).await?;

    // Additive migration for databases created before these columns existed.
    // `CREATE TABLE IF NOT EXISTS` never alters an existing table.
    ensure_column!(
        pool,
        "events",
        "timezone",
        "ALTER TABLE events ADD COLUMN timezone TEXT"
    );
    ensure_column!(
        pool,
        "calendars",
        "sync_token",
        "ALTER TABLE calendars ADD COLUMN sync_token TEXT"
    );
    ensure_column!(
        pool,
        "calendars",
        "access_role",
        "ALTER TABLE calendars ADD COLUMN access_role TEXT"
    );
    ensure_column!(
        pool,
        "tasks",
        "task_list_id",
        "ALTER TABLE tasks ADD COLUMN task_list_id TEXT"
    );

    // Per-component UI state used to live as `ui.*` rows in `settings`. Move it to
    // its dedicated table once; on later boots nothing matches and both statements
    // are no-ops (or clean up a stray `ui.*` row written through `update_setting`).
    migrate_legacy_ui_state(&pool).await?;

    Ok(pool)
}

/// Move legacy `ui.*` rows out of `settings` and into `ui_state`.
///
/// Runs in one transaction so a crash between the copy and the delete cannot
/// leave the row in both tables (or neither).
///
/// # Errors
///
/// Returns an error if the transaction fails.
async fn migrate_legacy_ui_state(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT OR IGNORE INTO ui_state (key, value)
         SELECT key, value FROM settings WHERE key LIKE 'ui.%'",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM settings WHERE key LIKE 'ui.%'")
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
