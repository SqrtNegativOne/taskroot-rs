# Backend Architecture & Rules

This file contains rules specific to the Tauri v2 Rust backend of Taskroot. It supplements the root `AGENTS.md`.

## Key Concepts
- **Storage Channels**: `settings.json` holds one `AppSettings` object (`get_settings`/`update_setting`); `ui-state.json` is a flat map for per-component UI state (`get_ui_state`/`set_ui_state`); `account.json` holds the server URL, principal/home-set and discovered collections but no password. Calendar data is not cached on disk. A stored setting is merged only if it round-trips through its field's serde type, so a wrongly-typed value keeps the default instead of corrupting the struct; `update_setting` rejects a *known* key whose value fails that probe with `invalid-input` (unknown keys stay generic). `store/persist.rs` owns the JSON read/write and merge rule. `app_data_dir` (`lib.rs`) falls back to `<exe dir>/dev-data` for debug builds whose app-data directory is unwritable (sandboxed dev runs), seeding the three files above; `TASKROOT_DATA_DIR` always wins. See the root `AGENTS.md` → Development Environment Notes.
- **Setting Metadata**: `get_settings_schema` is generated from `#[setting(..)]` attributes on `AppSettings`. `#[derive(SettingsMeta)]` (`src-tauri/taskroot-macros`) emits `AppSettings::setting_metadata()`, and each `defaultValue` is read from `AppSettings::default()`. Add metadata next to the field; never re-add a hand-written `json!` schema.
- **In-Memory Store**: `store::Store` maps every fetched resource href to `Resource { href, collection, etag, raw, items }`. `items` is `Vec<AppEvent>` or `Vec<AppTask>` according to the collection's `CalendarComponent`. Ids come from ICS content (`UID`, `UID@RECURRENCE-ID` for an exception), so a load needs no id lookup; `remote_id` is the href and `remote_collection_id` is the collection href. A new resource is created at `<collection>/<uid>.ics`. `StoreState` wraps the store in a `tokio::sync::Mutex`; commands lock it, merge and `PUT`.
- **Startup & Refresh**: `load::load_all` reads `account.json`, re-reads the collection list with one `PROPFIND` of the stored calendar home (`discovery::refresh_collections`: 3 s timeout, cached list on failure, persisted with `account::write_account` when it changed), then replaces the store contents with `ops::full_load`, which fetches every collection with one `calendar-query` each. It runs once at startup, after `connect_account`, and from the manual `refresh_data` command; a failed refresh leaves the previous resources in place. There is no poller, no sync tokens and no offline fallback.
- **Event Normalization Boundary**: `domain::event_timing` owns the only conversion from stored `AppEvent` wall-clock/all-day strings to `EventTiming` (tagged `AllDay`/`Timed`) and `EventInstance`. Naive timed strings are resolved in `AppEvent.timezone` (fallback UTC) via `wall_clock_to_utc`, so timing stays DST-correct. `domain::projection::expand` consumes those plus the requested range to produce instances, honoring `occurrenceKey`-based exception suppression, all-day floating-date recurrence, and cancellation tombstones; it passes `AppEvent.exdates` to the rule engine, which is the single home of exclusion dates (`rrule` holds `RRULE` lines only). Timed recurrence is expanded in the event's zone (`DTSTART;TZID=…`), then each occurrence is converted to UTC for `EventInstance`. Never parse `start_time`/`end_time` in commands or consumers.
- **ICS Read/Write Path**: `src-tauri/src/ics/` is the only CalDAV resource ↔ DTO boundary. `read_event_resource`/`read_task_resource` map a whole resource body with `icalendar`; `write_event_resource`/`write_task_resource` start from the fetched bytes and replace only the owned property lines (`SUMMARY`, `DESCRIPTION`, `DTSTART`, `DTEND`, `RRULE`, `EXDATE`, `STATUS`, `COLOR`, `X-TASKROOT-TASK-ID` for VEVENT; `SUMMARY`, `DESCRIPTION`, `DTSTART`, `DUE`, `RRULE`, `STATUS`, `COMPLETED`, `PERCENT-COMPLETE`, `PRIORITY`, `CATEGORIES`, `GEO`, `RELATED-TO;RELTYPE=PARENT`, `X-TASKROOT-*`, `CREATED`, `LAST-MODIFIED`, `DTSTAMP` and each `VALARM`'s `TRIGGER` for VTODO), so `VTIMEZONE`, `ATTENDEE`, `X-MOZ-GENERATION`, `X-APPLE-SORT-ORDER`, `SEQUENCE`, `LOCATION` and unknown `X-*` survive verbatim. `lines.rs` tracks `BEGIN`/`END` depth, so a nested component's property lines belong to it and never to its parent (a `VALARM`'s `DESCRIPTION` is not the task's `DESCRIPTION`); nested components are matched by position and only rewritten when the DTO's list changed, which keeps alarms Taskroot cannot model byte-identical. `delete_*_component` returns `None` when no component remains, which means DELETE the resource. Nothing may serialize a DTO into a resource. Canonical value formats are documented on `ics::read`; event ids are `<UID>` for a master and `<UID>@<RECURRENCE-ID>` for an exception. `write_event_resource` inserts a missing VEVENT (used to append exceptions), but refuses to mix UIDs in one resource. `store::ops::update_task` refuses to complete a task whose `RRULE` is set: tasks.org completes one by rolling the series forward, so a plain `STATUS:COMPLETED` would end it.
- **Calendar Write Path**: Every command edit is a read-modify-write of the stored raw resource. `ops::create_*` builds a new resource with `icalendar` and `PUT`s it with `If-None-Match: *`; `ops::update_*` merges into `resource.raw`, `PUT`s with `If-Match`, and on `412` re-fetches/re-merges/retries once (a second failure is returned, never swallowed). When the target collection changed, the new resource is `PUT` first and the old one deleted. Deletes use `DELETE`; `404` counts as success. A recurring event's edits take an `EventScope`: `ops::delete_event` and `ops::reschedule_event` resolve `this` (append `EXDATE` / write a `RECURRENCE-ID` exception), `following` (truncate `RRULE` before the occurrence via `domain::timing::truncate_before`, dropping later exceptions) and `all` (whole resource); the frontend passes the clicked `EventInstance::occurrence_start` and never builds rule text. `AppCalendar.writable` gates every write with `invalid-input`, and the frontend greys out read-only calendars. See `docs/caldav.md`.
- **Read-Only Semantics**: A discovered collection carries `writable` from `current-user-privilege-set`; no local-only storage mode exists. `Store::default_calendar` picks the requested writable collection of the right component, otherwise the first one. The account gate (`is_account_configured`) is repurposed as "connect your CalDAV server".

## Command Testing

Every `#[tauri::command]` handler is a **thin wrapper**, and the command layer is
tested at the seam underneath it:

- A handler marshals its arguments, resolves `store_state(&app)` and/or `client_for(&app)` and delegates. Behaviour lives below the command layer: `store::ops` (load/create/update/delete), `store::query` (filter/sort/search), `store::persist` (JSON), `settings::storage`, `load::*`, or an `AppHandle`-based helper in `commands::session`. Either way: no protocol or merge logic in a handler, and a new branch is a hint that the body wants a store-level function.
- The store tests (`store/tests.rs`, `store/query.rs`) cover full-load replacement, JSON merge/reject rules and the in-memory query path; `settings/tests.rs` drives the store-backed settings/UI-state bodies. Test code relies on `clippy.toml`'s `allow-unwrap-in-tests`/`allow-expect-in-tests` instead of per-module allows.
- `test_support::caldav::start_server()`/`start_http_server()` and `test_support::ics::{series, all_day, with_exception}` are the shared wiremock server and ICS fixture setup for the CalDAV and store tests; reuse them instead of starting servers or inlining `.ics` resources.
- `src-tauri/examples/e2e_caldav.rs` is the live Radicale check (event/task create → edit/move → delete, reload + body comparison, `X-APPLE-SORT-ORDER` preservation). Run it with `TASKROOT_DATA_DIR=target/e2e-data cargo run --example e2e_caldav -- --user … --password …`; it needs a running server and is not part of CI.

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
own `store_state(&app)`/`client_for(&app)` lines are only compile-checked.

## Style & Idioms
- **Rust Idioms**: Write clean, idiomatic Rust. Handle all `Result` and `Option` types safely (do not use `unwrap()` or `expect()` in production code unless absolutely necessary). Use `clippy` for linting.
- **Testing**: Backend tests should cover CalDAV XML parsers, the I1–I3 ICS invariants, store behaviour and `AppError` shape checks. Coverage is collected via `cargo-llvm-cov`. Run `cargo clippy --all-targets -- -D warnings` and `cargo nextest run` after changes. Command-layer changes follow the Command Testing convention above.
- **Serde Serialization**: Never use `#[serde(untagged)]` on enums, especially those with struct variants containing `Option<T>` fields. Untagged enums rely on structural "duck typing" which can cause silent parsing failures where an object parses as the wrong variant simply because it shares a few required fields (e.g. `id` and `title`). Use internally tagged enums (`#[serde(tag = "...")]`) or externally tagged enums instead.
