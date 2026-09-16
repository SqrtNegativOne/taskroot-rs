# Backend Architecture & Rules

This file contains rules specific to the Tauri v2 Rust backend of Taskroot. It supplements the root `AGENTS.md`.

## Key Concepts
- **Storage Channels**: Two tables back persisted state. `settings` holds the `AppSettings` fields (`get_settings`/`update_setting`) plus the `google_*` OAuth rows written by `auth.rs`; a stored value is merged only if it round-trips through its field's serde type, so a wrongly-typed row silently keeps the default instead of corrupting the struct. `update_setting` rejects a *known* key whose value fails that probe with `invalid-input` (unknown keys stay generic). Arbitrary per-component UI state (filters, sorts, view modes, pane sizes) lives in `ui_state` (`get_ui_state`/`set_ui_state`). `settings/storage.rs` owns the decode/encode and merge rule.
- **Setting Metadata**: `get_settings_schema` is generated from `#[setting(..)]` attributes on `AppSettings`. `#[derive(SettingsMeta)]` (`src-tauri/taskroot-macros`) emits `AppSettings::setting_metadata()`, and each `defaultValue` is read from `AppSettings::default()`. Add metadata next to the field; never re-add a hand-written `json!` schema. Custom button settings (`logout`, `clear_all_data`) live in `CUSTOM_SETTINGS`.
- **Database & Migrations**: All tasks and events are stored locally in an SQLite database (`taskroot.db`) located in the app data directory. The schema is created in `db::init_db` (`src-tauri/src/db/migrations.rs`) using inline SQL queries with `CREATE TABLE IF NOT EXISTS` guards, followed by additive `ensure_column!` migrations for columns added after a database was first created (`events.timezone`, `calendars.sync_token`, `calendars.access_role`, `tasks.task_list_id`). The legacy `ui.*` → `ui_state` copy runs inside a transaction so the copy and delete cannot interleave with a crash. `db/mod.rs` only wires submodules and `FilterColumnExt`; `db/tests.rs` holds the backend CRUD/migration tests.
- **Event Normalization Boundary**: `domain::event_timing` owns the only conversion from mirrored `AppEvent` wire strings to `EventTiming` (tagged `AllDay`/`Timed`) and `EventInstance`. `screens::plan::expand` consumes those plus the requested range to produce instances, honoring `occurrenceKey`-based exception suppression, all-day floating-date recurrence, and cancellation tombstones. Never parse `start_time`/`end_time` in commands or consumers.
- **Incremental Google Sync**: `apis/google_calendar/events.rs` stores a per-calendar `syncToken`, requests `showDeleted=true`, and resets to a full `timeMin` window on HTTP 410 Gone. Deleted standalone events/master series are removed locally; cancelled instances are retained as exceptions. Each calendarList entry's `accessRole` is mirrored to `AppCalendar.accessRole`; the frontend greys out editing for `reader`/`freeBusyReader` calendars. Google Tasks have no sync token, so every list is fetched with `showDeleted` + pagination and its `list_id` persisted.
- **Calendar Write Path**: `apis/google_calendar/write.rs` is the pure `&AppEvent` → (method, URL, JSON body) seam used by `publish()`. Existing events (`remote_id` set) are `PATCH`ed with `events.patch`, not `PUT`/`events.update`: `PUT` is a documented full replace that clears unmodeled fields. `recurrence` is rebuilt by splitting `AppEvent.rrule` on `\n`; `timeZone` comes from `AppEvent.timezone` for timed events. Event `color` is read-only (Google writes `colorId`, the app stores resolved hex). Exception instances (`recurringEventId`/`originalStartTime`) are sent only on insert, since both fields are immutable. See `docs/google-calendar-write.md`.
- **Deliberate Dead Code**: `get_dirty_tasks`/`get_dirty_events` and some `SyncQueue` methods carry `#[allow(dead_code)]`; they are reserved for the planned offline-enqueue roadmap. Do not delete them.

## Command Testing

Every `#[tauri::command]` handler is a **thin wrapper**, and the command layer is
tested at the seam underneath it:

- A handler marshals its arguments, resolves `db_pool(&app)` and delegates. SQL lives
  behind a `&SqlitePool` function: `db::*`, `settings::storage`, `sync::*`, or — when
  the body would otherwise be a couple of statements — a pool-level function next to
  the handler (`commands::tasks::past_due_task_ids`, `sync::queue_payloads`). Behaviour
  that genuinely needs the `AppHandle` (sync push/enqueue, window and stopwatch state)
  stays in the handler as a call sequence over those functions. Either way: no SQL in a
  handler, and a new branch is a hint that the body wants a pool-level function.
- `test_support::in_memory_pool()` returns a `SqlitePool` with the production schema
  and `test_support::{task, event}` are the row fixtures for it. It is test-only
  (`#[cfg(test)]`); never let test helpers reach a release build.
- Smoke tests drive that pool-level body against the in-memory database:
  `settings/tests.rs` (`get_settings` returns the defaults on an empty table,
  `update_setting` → `read_settings` round trip, a wrongly-typed known setting is
  rejected without writing, plus the `ui_state` round trip) and `commands/tests.rs`
  (`commands::tasks::past_due_task_ids`, and the `db::get_calendars` delegation behind
  `commands::events::get_active_calendars`). Their modules carry
  `#![allow(clippy::unwrap_used, clippy::expect_used)]`, like `db/tests.rs`.

**Why there is no mock-runtime test.** A handler can only be invoked end-to-end through
`tauri::test::get_ipc_response`, which needs a `Webview<MockRuntime>`. The commands
that take `app` use the concrete `tauri::AppHandle` (`AppHandle<Wry>`), and `MockRuntime`
cannot satisfy that parameter — `AppHandle<Wry>: CommandArg<'_, MockRuntime>` does not
hold, so the production `generate_handler!` table cannot even be installed into a mock
app. Making the 33 commands that take `app` generic over `R: Runtime` would rewrite
every public command signature, which the thin-wrapper rule does not cover. On Windows
there is a second blocker: `tauri-build` attaches the Common Controls v6 manifest to binary targets only
(`rustc-link-arg-bins`), so a test binary that builds an `App` aborts at load with
`STATUS_ENTRYPOINT_NOT_FOUND` no matter what the handler does.

Instead the JS ↔ Rust contract is pinned by source scans (`test_support::source_scan`
for the Rust side, `test_support::frontend_scan` for the call sites), asserted in
`commands/tests.rs`:

- every command in `generate_handler!` is listed once and is snake_case;
- the registered names and the `#[tauri::command]` functions under `src` are the same set;
- every literal command name at a `safeInvoke`/`useAutoQuery`/`useTauriQuery`/`invoke`
  call site under `../src` is registered, and every argument key it passes matches a
  camelCased Rust parameter of that command (the exact key `#[tauri::command]` looks up).
  A `useTauriQuery(..)` result bound to a local is followed to its later
  `query.execute({..})` calls, so those keys are checked too.

Limits to keep in mind: the scan sees literal command names only (a call site that
computes the name is not checked) and it skips test sources (`src/test/**`, `*.test.ts`,
`*tests.rs`). The source heuristics are deliberately shallow and can mis-read
non-literal code:

- `between(.., ']')` stops at the *first* `]`, so a `]` inside the `generate_handler!`
  list would truncate the registered-command scan;
- `#[tauri::command]` parameters are split on every comma, so a parameter type that
  contains a comma (e.g. a generic) would be mis-split;
- only that the *supplied* keys are declared is checked — a call site that omits a
  required argument is not flagged (the handler would reject it at runtime).

A green suite is a contract check, not end-to-end coverage of a handler — the handler's
own `db_pool(&app)` line is only compile-checked.

## Style & Idioms
- **Rust Idioms**: Write clean, idiomatic Rust. Handle all `Result` and `Option` types safely (do not use `unwrap()` or `expect()` in production code unless absolutely necessary). Use `clippy` for linting.
- **Testing**: Backend tests should cover migration assertions and `AppError` shape checks. Coverage is collected via `cargo-llvm-cov`. Run `cargo clippy --all-targets -- -D warnings` and `cargo nextest run` after changes. Command-layer changes follow the Command Testing convention above.
- **Serde Serialization**: Never use `#[serde(untagged)]` on enums, especially those with struct variants containing `Option<T>` fields. Untagged enums rely on structural "duck typing" which can cause silent parsing failures where an object parses as the wrong variant simply because it shares a few required fields (e.g. `id` and `title`). Use internally tagged enums (`#[serde(tag = "...")]`) or externally tagged enums instead.
