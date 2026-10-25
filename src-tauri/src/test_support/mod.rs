//! Shared support for the command-layer tests (see `AGENTS.md` → Command Testing).
//!
//! [`source_scan`]/[`frontend_scan`] pin the JS ↔ Rust command contract by reading
//! the sources.
//!
//! There is deliberately no mock-Tauri helper here. A `#[tauri::command]` handler
//! can only be invoked through `tauri::test::get_ipc_response`, which needs an
//! `App<MockRuntime>`, and the commands that take `app` use the concrete
//! `tauri::AppHandle` (i.e. `AppHandle<Wry>`) — `MockRuntime` cannot satisfy that
//! parameter. The store/ICS/CalDAV layers are tested directly instead, and
//! [`source_scan`] plus [`frontend_scan`] check the handler table against the
//! frontend call sites.
#![allow(clippy::unwrap_used, clippy::expect_used)]

pub mod frontend_scan;
pub mod source_scan;
