# Architecture map

Taskroot is a desktop task manager for planning, working and resting. The frontend
is Svelte 5 + TypeScript, with SvelteKit's static adapter in SPA mode (`ssr = false`),
Vite on port 1420, vanilla CSS and Bun. The native backend is Tauri v2 + Rust.

This is a navigation reference, not a file-by-file inventory. Read the applicable
[root](../AGENTS.md), [frontend](../src/AGENTS.md) or
[backend](../src-tauri/AGENTS.md) instructions before editing.

## Data and ownership

- One CalDAV server (normally local Radicale) owns calendar/task data. The Rust
  store keeps fetched resources, ETags, raw ICS and typed rows in memory. There is
  no on-disk calendar cache, offline write queue or background calendar sync.
- Startup, account connection and manual Refresh load collections. Network fetches
  run outside the store lock; a resource-revision guard protects application from
  tracked concurrent writes. Collection mutations have coverage gaps listed in
  [known limitations](known-limitations.md). Writes merge into raw ICS and go to
  the server immediately. See [CalDAV](caldav.md).
- `settings.json` holds typed `AppSettings`; `ui-state.json` holds per-component
  state. `account.json` holds connection/discovery information, not the password;
  credentials use the OS keychain. `TASKROOT_CALDAV_PASSWORD` can override the
  password for headless/development use. See [Windows development](windows-development.md)
  for the data-directory override and debug fallback.
- `src/lib/bindings/*.generated.ts` comes from Rust `ts-rs` export tests;
  `src/lib/domain.ts` is the frontend domain barrel.

## Frontend entry points

| Area | Start here |
|---|---|
| SPA/window routing, toasts and daily startup redirect | `src/routes/+layout.svelte`, `src/lib/dailyStartup.ts`, `src/lib/routes.ts` |
| Routing/chunk-load recovery | `src/routes/+error.svelte` |
| Settings and CalDAV account/list management | `src/routes/settings/`, `src/components/CalendarManager.svelte` |
| Backend mutations, queries and persistence | `src/lib/store.svelte.ts`, `src/lib/safeInvoke.svelte.ts`, `src/lib/queries.ts`, `src/lib/persisted.svelte.ts`, `src/lib/asyncState.svelte.ts` |
| Shared task-list definition | `src/lib/taskList.svelte.ts`, `src/lib/TaskListState.svelte`, `src/components/tasklist/` |
| Task/event inspector, recurrence scope, event-to-task conversion | `src/components/inspector-pane/` |
| Recursive filter/sort UI and defaults | `src/components/filters/`, `src/components/FilterButton.svelte`, `src/lib/domain/filterTree.ts`, `src/lib/domain/filters.ts` |
| Planning, spanning events and drag/drop | `src/screens/plan/date-grid/`, `src/screens/plan/day-timeline/`, `src/screens/plan/hooks/`, `src/screens/plan/snapping.ts` |
| Do screen and shared stopwatch feedback | `src/screens/do/stopwatch/` |
| Quick-capture palette | `src/lib/launcher/LauncherPalette.svelte` |
| Mini tracker dragging/menu geometry | `src/routes/minitracker/controller.svelte.ts`, `src/routes/minitracker/menuLayout.ts` |
| Sidebar notes/tab position | `src/screens/sidebar/state.svelte.ts` |

[Frontend reference](frontend.md) explains the behavior shared across these areas;
[test setup](testing.md) documents the existing fixtures and mocks.

## Backend entry points

All backend paths below are under `src-tauri/src/`.

| Area | Start here |
|---|---|
| Module/lint wiring | `lib.rs` |
| Tauri composition, plugins, setup and command registration | `bootstrap/` |
| Desktop windows, tray and data-directory resolution | `platform/` |
| Command wrappers and Tauri calendar-session adapter | `ipc/`, especially `session.rs`; contract tests in `ipc/tests.rs` |
| Calendar domain, transport, ICS, in-memory store and projections | [Calendar feature](../src-tauri/src/features/calendar/README.md) |
| Shared clock, persisted focus/history and optional localhost feed | [Stopwatch feature](../src-tauri/src/features/stopwatch/README.md), [focus-feed API](focus-feed.md) |
| Task alarm policy and notification polling | [Reminders feature](../src-tauri/src/features/reminders/README.md) |
| Typed settings, field metadata and persistence adapter | `settings/mod.rs`, `settings/metadata.rs`, `settings/storage.rs`; derive in `../taskroot-macros/` |
| Headless CLI | `bin/taskroot-cli.rs`, `cli/`; [generated CLI reference](cli.md) |
| Shared fixtures and source-contract scans | `test_support/`; [testing reference](testing.md) |

The CLI loads a private store for data commands and shares calendar query/write
operations with the app. It does not parse or serialize ICS itself. Its writes
appear in an already-running app on Refresh/restart, not through cross-process
notifications. `cli/help.rs` owns the verb/option tables and generates `docs/cli.md`
from a test; do not maintain a second verb inventory here.

## Windows and stopwatch focus

Five window labels are configured in `src-tauri/tauri.conf.json`: `main`,
`launcher`, `minitracker`, `sidebar` and `effect`. They render the same SPA,
branched by window label. Launcher, mini tracker and effect windows are created
lazily from that config. Main hides to tray on close; the effect window is a
transparent click-through phase-transition overlay.

The launcher shortcut is `CommandOrControl+Shift+Space`. The mini tracker chooses
a task/custom label and controls the shared Rust stopwatch. That focus is app-local
(`stopwatch-focus.json`), independent of VTODO status. The Do screen displays it
read-only, resolving the live task title from an unfiltered query with the shared
label as fallback. Clock phases and cross-window notifications come from Rust,
not separate frontend clocks. Main-window audio/overlay feedback lives in
`src/screens/do/stopwatch/feedback.ts`.

## Adding a setting

Declare the `AppSettings` field, its default and a `#[setting(..)]` attribute in
`src-tauri/src/settings/mod.rs`. `SettingsMeta` derives the schema and reads default
values from `AppSettings::default()`; a field without metadata fails compilation.
Add new sections to `SETTING_SECTIONS` in `settings/metadata.rs`. Field order sets
row order and section order sets tab order.

`kind = "calendar"` renders a component-specific writable-list select.
`default_task_list`/`default_event_list` store collection hrefs; an empty value means
the first writable collection of the right kind. Regenerate bindings after changes.

`store/persist.rs` deserializes the whole settings file: malformed or wrongly typed
settings fall back to the complete default object (with a `.bad` rename for parse
failure). `apply_setting` rejects a known key with an invalid type; unknown keys
are accepted but ignored. This is not an arbitrary settings-key store—use UI state
for per-component values.
