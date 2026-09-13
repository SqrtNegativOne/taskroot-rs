//! Shared support for the command-layer tests (see `AGENTS.md` → Command Testing).
//!
//! [`in_memory_pool`] is the paved road for exercising a command body against a
//! real database; [`task`]/[`event`] are the fixtures those tests start from, and
//! [`source_scan`]/[`frontend_scan`] pin the JS ↔ Rust command contract by reading
//! the sources.
//!
//! There is deliberately no mock-Tauri helper here. A `#[tauri::command]` handler
//! can only be invoked through `tauri::test::get_ipc_response`, which needs an
//! `App<MockRuntime>`, and the commands that take `app` use the concrete
//! `tauri::AppHandle` (i.e. `AppHandle<Wry>`) — `MockRuntime` cannot satisfy that
//! parameter. Making those 33 commands runtime-generic would change 33 public
//! signatures, which this wave does not do. So the smoke tests drive the
//! `&SqlitePool` body each command delegates to, and [`source_scan`] plus
//! [`frontend_scan`] check the handler table against the frontend call sites.
#![allow(clippy::unwrap_used, clippy::expect_used)]

pub mod frontend_scan;
pub mod source_scan;

use crate::domain::{AppEvent, AppTask, AppTaskStatus, EventStatus};
use sqlx::SqlitePool;

/// An in-memory database with the production schema applied by [`crate::db::init_db`].
pub(crate) async fn in_memory_pool() -> SqlitePool {
    crate::db::init_db("sqlite::memory:")
        .await
        .expect("in-memory database initialises")
}

/// A minimal open task. Override fields with struct-update syntax in the test.
pub(crate) fn task(id: &str, title: &str) -> AppTask {
    AppTask {
        id: id.into(),
        title: title.to_string(),
        status: Some(AppTaskStatus::Todo),
        priority: None,
        tags: None,
        checklist: None,
        parent_task: None,
        dependencies: None,
        est: None,
        added: None,
        canvas_x: None,
        canvas_y: None,
        on_canvas: None,
        remote_id: None,
        notes: None,
        tabs: None,
        due: None,
        updated_at: None,
        etag: None,
        dirty: None,
        task_list_id: None,
    }
}

/// A minimal confirmed timed event ending at `end_time` and linked to no task.
pub(crate) fn event(id: &str, end_time: &str) -> AppEvent {
    AppEvent {
        id: id.into(),
        remote_id: None,
        remote_collection_id: None,
        task_id: None,
        title: id.to_string(),
        description: None,
        start_time: end_time.to_string(),
        end_time: end_time.to_string(),
        rrule: None,
        exdates: None,
        recurring_event_id: None,
        original_start_time: None,
        status: Some(EventStatus::Confirmed),
        updated_at: None,
        color: None,
        etag: None,
        dirty: None,
        is_all_day: Some(false),
        timezone: None,
    }
}
