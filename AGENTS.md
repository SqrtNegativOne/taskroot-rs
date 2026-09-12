# Taskroot-rs Architecture & Guide for AI Agents

**CRITICAL**: When you modify the architecture, tech stack, or file structure of this project, you MUST update this `AGENTS.md` file to reflect the new state. Always verify if the information here is outdated and update any old information if needed.

*Note: Domain-specific rules are located in `src/AGENTS.md` (for Svelte) and `src-tauri/AGENTS.md` (for Rust).*

Taskroot is a desktop task management app focusing on planning, executing, and resting. It is built as a Svelte 5 application running on a native Tauri v2 Rust backend.

## Tech Stack
- **Package Manager**: Bun (`bun`). Used for package management and running frontend scripts.
- **Frontend Framework**: Svelte 5 with SvelteKit configured for SPA (Single Page Application) mode (`ssr = false` in `src/routes/+layout.ts`, `adapter-static` with an `index.html` fallback). Uses Svelte runes for reactivity.
- **Build Tool**: Vite (via SvelteKit), fixed dev port 1420 (`vite.config.js`).
- **Desktop Wrapper**: Tauri v2 (Rust backend, configured in `src-tauri/tauri.conf.json`).
- **Language**: TypeScript (`.ts`, `.svelte`) on the frontend, Rust (`.rs`) on the backend.
- **Styling**: Vanilla CSS (`src/app.css`) with extensive use of CSS variables for theming.
- **Backend / Storage**: Local SQLite database managed by Rust (`sqlx`, with the `migrate` feature). Schema is applied via inline CREATE statements in `db::init_db`. Data is queried via Tauri IPC commands.
- **Type Sharing**: `ts-rs` generates `src/lib/bindings/*.generated.ts` from Rust structs; `cargo nextest run` is the regeneration trigger (see Key Concepts).
- **Testing**: 
  - **Frontend E2E**: Playwright (`playwright.config.ts`, specs in `tests/e2e/`, run via `bun run test`; the config boots `bun run dev` on port 1420 with chromium).
  - **Frontend Unit**: Vitest with jsdom (`bun run test:unit`). Coverage is collected via `@vitest/coverage-v8` (`bun run coverage:frontend`).
  - **Backend**: Rust unit tests (`cargo nextest run` in `src-tauri`, including migration assertions and `AppError` shape checks). Coverage is collected via `cargo-llvm-cov` (`bun run coverage:backend`, requires `cargo install cargo-llvm-cov`).
- **Linters**: ESLint (strictTypeChecked; `**/*.generated.ts` is exempt from `array-type` and `consistent-type-definitions` in `eslint.config.js`) and Rust `clippy` (`lib.rs` warns on pedantic/nursery and denies `unwrap`/`expect`). CI (`.github/workflows/ci.yml`) runs `bun run check` + `bun run lint` and `cargo clippy --all-targets -- -D warnings` + `cargo nextest run`. **CRITICAL: You must run `bun run check` (for frontend) and `cargo clippy` (for backend) after EVERY change to ensure code quality and avoid regressions.**

## Project Structure
- `src/`: SvelteKit frontend codebase.
  - `src/routes/`: SvelteKit routing (`+layout.svelte`, `+page.svelte`). The single SPA branches on the Tauri window label (`main`, `launcher`, `minitracker`) inside `+layout.svelte`.
  - `src/routes/settings/`: Settings screen split into `schema.ts` (typed schema consuming the generated `AppSettings` type) and `SettingRow.svelte`.
  - `src/lib/`: Shared logic and Svelte runes:
    - `store.svelte.ts`: Primary store. Mutations return `neverthrow` `Result`s, await the backend command, then re-fetch raw state from SQLite. `init()` is idempotent (one cached bootstrap promise) and reads `get_settings` once on mount.
    - `safeInvoke.svelte.ts`: `safeInvoke` (`ResultAsync` wrapper over `invoke`) and the `useTauriQuery` rune (stale-guard via request IDs, optional `debounceMs`).
    - `errors.ts`: Typed `BackendErrorCode`/`AppError` glue mirroring the Rust `AppError` contract (`normalizeAppError`, `describeAppError`).
    - `events.ts`: TypeScript mirrors of the backend event-name constants in `src-tauri/src/events.rs`.
    - `eventColor.ts`: Shared event-accent presentation (`eventColorVars`) emitting the `--ev-color`/`--ev-bg` custom properties both the date grid and day timeline consume, so their colors cannot diverge.
    - `time.ts`: Local-date helpers (`ymd`, `addDays`, `dayDiff`, `sameDay`). Never use `toISOString()` for day bucketing (it shifts to UTC).
    - `useNow.svelte.ts`: Shared reactive `now` primitive (one interval, cleaned up automatically); use it instead of ad-hoc rAF loops.
    - `persisted.svelte.ts`: `persistState` rune + `persistedKeys` that hydrate per-component UI state (filters, sorts, view modes, pane sizes, collapsible sections) from the backend `ui_state` table via `get_ui_state` and debounce-save changes via `set_ui_state`, flushing pending writes on destroy.
    - `routes.ts`: Centralized route-path constants.
    - `domain.ts`: Centralized domain barrel re-exporting generated bindings from `src/lib/bindings/` and domain models/events/timing/filters (modularized in `src/lib/domain/models.ts`, `events.ts`, `timing.ts`, `filters.ts`). `timing.ts` provides the only render-path day-overlap helpers (`instanceOccursOnDay`, `isAllDayTiming`).
  - `src/lib/bindings/`: Generated TypeScript bindings (`.generated.ts`) for Rust data structures (generated via `ts-rs` by running `cargo nextest run` in `src-tauri`). Never hand-edit.
  - `src/screens/`: Major UI views. `plan/` (with `day-timeline/`, including `hooks/pointerGesture.svelte.ts` — a window-pointer gesture registry with `pointercancel` and teardown safety — and `date-grid/`) and `do/` (with `stopwatch/`, whose `engine.svelte.ts` consumes the generated `StopwatchState`).
  - `src/components/`: Reusable UI components. `ComingSoon.svelte` consolidates the seven stub route pages; `inspector-pane/` is split into `InspectorPane`, `InspectorTaskFields`/`InspectorEventFields`, and `format.ts`.
  - `src/test/`: Shared test utilities (`tauriMock.ts`, the process-wide Tauri IPC mock used by component tests). See `src/AGENTS.md` for the testing conventions.
- `src-tauri/`: Tauri Rust backend.
  - `src-tauri/src/lib.rs`: Lints, module wiring, `db_pool()`, and `run()`. IPC commands live in `commands/`, not here.
  - `src-tauri/src/commands/`: IPC command handlers split by domain (`tasks.rs`, `events.rs`, `window.rs`, `sync.rs`).
  - `src-tauri/src/error.rs`: `AppError` enum (`thiserror`) returned by ALL IPC commands; serialized as `{code, message}`.
  - `src-tauri/src/events.rs`: Centralized event-name constants (`STOPWATCH_UPDATED`, `SYNC_STARTED/FINISHED/ERROR`, `OAUTH_URL`).
  - `src-tauri/src/db/`: Modularized SQLite operations using `sqlx` (`tasks.rs`, `events.rs`, `settings.rs`, `ui_state.rs`, plus `task_filters.rs` for the dynamic `QueryBuilder` filtering path). `mod.rs` declares the submodules and `FilterColumnExt`; `migrations.rs` holds `init_db` (inline `CREATE TABLE IF NOT EXISTS` plus additive `ensure_column!` migrations so older databases gain `events.timezone`, `calendars.sync_token`, and `tasks.task_list_id`, then a transactional migration of legacy `ui.*` rows from `settings` into `ui_state`); backend CRUD tests live in `tests.rs`.
  - `src-tauri/src/domain/`: Core data structures (`mod.rs`, `sigil.rs` for sigil parsing, `filters.rs` for filter columns/types) plus `event_timing.rs` — the **single normalization boundary** (`EventTiming`, `EventInstance`, `EventTiming::from_event`) that turns mirrored wire strings into tagged all-day/timed timing. Nothing outside this module may parse `AppEvent::start_time`/`end_time`.
  - `src-tauri/src/apis/`: Google API clients. `google_calendar/` is split into `mod.rs` (publish/delete transport), `write.rs` (the pure `&AppEvent` → method/URL/body builder; updates use `events.patch`, creates use `events.insert`), `types.rs`, and `events.rs` (incremental `syncToken` list with `showDeleted` and 410-Gone reset); `google_tasks.rs` syncs every task list with `showDeleted` + pagination. Write semantics and per-occurrence gaps are documented in `docs/google-calendar-write.md`.
  - `src-tauri/src/sync/`: Global sync engine: `mod.rs` (5-minute poller, `SyncState`), `push.rs` (enqueue + `plan_event_sync`, which turns a calendar change into a `SyncAction::Move`), `drain.rs` (drains the queue: publish/move/delete, surfaces the first push failure), `types.rs`, and the offline queue (`queue.rs`, `queue_store.rs`).
  - `src-tauri/src/stopwatch.rs`: Stopwatch backend (`StopwatchState` struct plus `get/toggle/reset_stopwatch` commands).
  - `src-tauri/src/settings/`: Settings backend, split into `mod.rs` (`AppSettings` struct with `#[derive(TS)]` and `#[derive(SettingsMeta)]` field-adjacent metadata plus the IPC commands), `storage.rs` (JSON decode/encode, the per-field serde merge rule, `apply_stored_settings`), and `metadata.rs` (`SettingMeta`/`SettingOption`, `SETTING_SECTIONS`, `CUSTOM_SETTINGS`, and `build_settings_schema`, with tests in `metadata/tests.rs`). `get_settings_schema` is generated from the field attributes, and the binding is emitted to `src/lib/bindings/AppSettings.generated.ts`. `get_settings` merges stored rows over the defaults with a serde round-trip per field (a wrongly-typed stored value keeps the default), and `update_setting` persists one typed setting. The `settings` table holds the `AppSettings` fields plus the `google_*` OAuth rows written by `auth.rs`; arbitrary per-component UI state lives in the separate `ui_state` table via `get_ui_state`/`set_ui_state`.

### Key Concepts
- **Typed Error Contract**: Every IPC command returns `Result<T, AppError>`. `AppError` serializes as `{code, message}` with kebab-case codes: `db`, `not-found`, `auth`, `sync`, `invalid-input`, `internal`. The frontend mirror lives in `src/lib/errors.ts` (`BackendErrorCode`). Never return raw strings from commands.
- **Event Name Constants**: Backend event names are constants in `src-tauri/src/events.rs`, mirrored in `src/lib/events.ts` (`stopwatch-updated`, `sync-started`, `sync-finished`, `sync-error`, `oauth-url`). Never inline raw event strings on either side.
- **Generated Bindings Flow**: `cargo nextest run` in `src-tauri` regenerates `src/lib/bindings/*.generated.ts` via `ts-rs` (`export_bindings_*` tests). After changing any `#[derive(TS)]` struct, run `cargo nextest run` and commit the regenerated files. CI fails on binding drift (`git diff --exit-code src/lib/bindings`). The `#[ts(type = "number")]` convention keeps timestamps as JS `number` — never `bigint`.
- **Adding a Setting**: Declare the field on `AppSettings` with a `#[setting(label = ..., kind = ..., section = ..., ...)]` attribute (`src-tauri/src/settings/mod.rs`). `#[derive(SettingsMeta)]` turns those attributes into `get_settings_schema`'s output and derives each `defaultValue` from `AppSettings::default()`, so a setting is described once. A field without `#[setting(..)]` fails to compile; a new section is added to `SETTING_SECTIONS`. Field order sets the row order inside a section, and `SETTING_SECTIONS` order sets tab/section order. Run `cargo nextest run` to regenerate the `AppSettings` TS binding.
- **Event Instance Projection**: Mirrored `AppEvent` rows stay wire-shaped for the push path. The render path consumes `EventInstance`s produced by `screens::plan::expand` and returned by `query_event_instances`. All-day occurrences are floating dates (`startDate`/`endDateExclusive`) expanded independently of the process timezone; timed occurrences carry UTC instants plus the Google `timeZone`. `occurrenceKey` (`masterId@<date|rfc3339>`) is the stable per-occurrence identity and the key for exception/cancellation suppression. `query_events` still returns raw `AppEvent`s for the inspector; drag/resize calls `reschedule_event`.
- **Multi-Window Architecture**: Three windows are declared in `tauri.conf.json`:
  - **Main Window** (`main`): The primary Svelte app; hides to tray on close.
  - **Launcher Window** (`launcher`): A spotlight-like command palette triggered via the global-shortcut plugin.
  - **Mini Tracker Window** (`minitracker`): A minimal window for tracking time. It runs independently but reads state from the Rust backend.
  All three render the same SPA, branched by window label in `+layout.svelte`.
- **Cross-Window State**: There is no dedicated inter-window wiring module. Window-specific state (stopwatch, sync status) lives in Rust and is synced across windows through Tauri commands (`get_stopwatch_state`, `toggle_stopwatch`, `reset_stopwatch`, `get_sync_state`) plus backend-emitted events consumed via `listen` with the `src/lib/events.ts` constants.

## Style (Important)
- **Generated Code**: Never hand-edit anything in `src/lib/bindings/` — regenerate via `cargo nextest run`.
- **Test-Driven Development**: Write tests first as a contract. Do not modify them unless there is something truly wrong.
- **Self-Documenting Code**: Avoid redundant comments. Extract complex logic into well-named functions or constants.
- **Small, Modular Code**: Refactor files if they exceed 250 LOC. Refactor functions with more than 4 levels of indentation.
- **Store Assets Offline**: Store assets offline.
- **Calendar Write Semantics**: Event edits use Google's `events.patch` (existing `remote_id`) or `events.insert` (new). Never use `events.update`/`PUT`: it is a full replace that wipes unmodeled fields. `recurrence` is rebuilt from `AppEvent.rrule` lines and `timeZone` from `AppEvent.timezone`; `color` is read-only (hex is not a `colorId`). Drag/resize (`reschedule_event`) must enqueue through `sync::push::push_or_enqueue`, like `update_event`.
- Tautological tests are considered harmful.
- Please use early returns. This will make code nicer for you too.