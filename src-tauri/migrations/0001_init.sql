-- Baseline schema, captured verbatim from the former inline CREATE statements
-- in db::init_db. New databases receive this schema directly.
--
-- IF NOT EXISTS is deliberate: databases created before migrations existed
-- have no _sqlx_migrations table, so 0001 executes against them on the first
-- MIGRATOR.run() and must no-op instead of failing.

CREATE TABLE IF NOT EXISTS tasks (
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
    deleted BOOLEAN,
    updated_at INTEGER,
    etag TEXT,
    dirty BOOLEAN DEFAULT 0
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
    cancelled BOOLEAN,
    updated_at INTEGER,
    deleted BOOLEAN,
    etag TEXT,
    dirty BOOLEAN DEFAULT 0
);

CREATE TABLE IF NOT EXISTS settings (
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
