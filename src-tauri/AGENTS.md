# Backend Architecture & Rules

This file contains rules specific to the Tauri v2 Rust backend of Taskroot. It supplements the root `AGENTS.md`.

## Key Concepts
- **Database & Migrations**: All tasks and events are stored locally in an SQLite database (`taskroot.db`) located in the app data directory. The schema is created in `db::init_db` using inline SQL queries with `CREATE TABLE IF NOT EXISTS` guards.
- **Deliberate Dead Code**: `get_dirty_tasks`/`get_dirty_events` and some `SyncQueue` methods carry `#[allow(dead_code)]`; they are reserved for the offline-enqueue roadmap (see `TODO.md`). Do not delete them.

## Style & Idioms
- **Rust Idioms**: Write clean, idiomatic Rust. Handle all `Result` and `Option` types safely (do not use `unwrap()` or `expect()` in production code unless absolutely necessary). Use `clippy` for linting.
- **Testing**: Backend tests should cover migration assertions and `AppError` shape checks. Coverage is collected via `cargo-llvm-cov`. Run `cargo clippy --all-targets -- -D warnings` and `cargo test` after changes.
