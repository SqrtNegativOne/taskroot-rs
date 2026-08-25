# Taskroot-rs Architecture & Guide for AI Agents

**CRITICAL**: When you modify the architecture, tech stack, or file structure of this project, you MUST update this `AGENTS.md` file to reflect the new state. Always verify if the information here is outdated and update any old information if needed.

Taskroot is a desktop task management app focusing on planning, executing, and resting. It is built as a Svelte 5 application running on a native Tauri v2 Rust backend.

## Tech Stack
- **Package Manager**: Bun (`bun`). Used for package management and running frontend scripts.
- **Frontend Framework**: Svelte 5 with SvelteKit configured for SPA (Single Page Application) mode (`ssr = false` in `src/routes/+layout.ts`, `adapter-static` with an `index.html` fallback). Uses Svelte runes for reactivity.
- **Build Tool**: Vite (via SvelteKit), fixed dev port 1420 (`vite.config.js`).
- **Desktop Wrapper**: Tauri v2 (Rust backend, configured in `src-tauri/tauri.conf.json`).
- **Language**: TypeScript (`.ts`, `.svelte`) on the frontend, Rust (`.rs`) on the backend.
- **Styling**: Vanilla CSS (`src/app.css`) with extensive use of CSS variables for theming.
- **Backend / Storage**: Local SQLite database managed by Rust (`sqlx`, with the `migrate` feature). Schema is applied via inline CREATE statements in `db::init_db`. Data is queried via Tauri IPC commands.
- **Type Sharing**: `ts-rs` generates `src/lib/bindings/*.generated.ts` from Rust structs; `cargo test` is the regeneration trigger (see Key Concepts).
- **Testing**: Playwright E2E (`playwright.config.ts`, specs in `tests/e2e/`, run via `bun run test`; the config boots `bun run dev` on port 1420 with chromium) and Rust unit tests (`cargo test` in `src-tauri`, currently 32 passing, including migration assertions and `AppError` shape checks).
- **Linters**: ESLint (strictTypeChecked; `**/*.generated.ts` is exempt from `array-type` and `consistent-type-definitions` in `eslint.config.js`) and Rust `clippy` (`lib.rs` warns on pedantic/nursery and denies `unwrap`/`expect`). CI (`.github/workflows/ci.yml`) runs `bun run check` + `bun run lint` and `cargo clippy --all-targets -- -D warnings` + `cargo test`. **CRITICAL: You must run `bun run check` (for frontend) and `cargo clippy` (for backend) after EVERY change to ensure code quality and avoid regressions.**

## Project Structure
- `src/`: SvelteKit frontend codebase.
  - `src/routes/`: SvelteKit routing (`+layout.svelte`, `+page.svelte`). The single SPA branches on the Tauri window label (`main`, `launcher`, `minitracker`) inside `+layout.svelte`.
  - `src/routes/settings/`: Settings screen split into `schema.ts` (typed schema consuming the generated `AppSettings` type) and `SettingRow.svelte`.
  - `src/lib/`: Shared logic and Svelte runes:
    - `store.svelte.ts`: Primary store. Mutations return `neverthrow` `Result`s, await the backend command, then re-fetch raw state from SQLite. `init()` is idempotent and retries on the `'not-ready'` error code while the async DB pool spins up.
    - `safeInvoke.svelte.ts`: `safeInvoke` (`ResultAsync` wrapper over `invoke`) and the `useTauriQuery` rune (stale-guard via request IDs, optional `debounceMs`).
    - `errors.ts`: Typed `BackendErrorCode`/`AppError` glue mirroring the Rust `AppError` contract (`normalizeAppError`, `describeAppError`).
    - `events.ts`: TypeScript mirrors of the backend event-name constants in `src-tauri/src/events.rs`.
    - `time.ts`: Local-date helpers (`ymd`, `addDays`, `dayDiff`, `sameDay`). Never use `toISOString()` for day bucketing (it shifts to UTC).
    - `useNow.svelte.ts`: Shared reactive `now` primitive (one interval, cleaned up automatically); use it instead of ad-hoc rAF loops.
    - `routes.ts`: Centralized route-path constants.
    - `domain.ts`: Barrel re-exporting the generated types from `src/lib/bindings/` (including `StopwatchState`, `SyncState`).
  - `src/lib/bindings/`: Generated TypeScript bindings (`.generated.ts`) for Rust data structures (generated via `ts-rs` by running `cargo test` in `src-tauri`). Never hand-edit.
  - `src/screens/`: Major UI views. `plan/` (with `day-timeline/`, including `hooks/pointerGesture.svelte.ts` — a window-pointer gesture registry with `pointercancel` and teardown safety — and `date-grid/`) and `do/` (with `stopwatch/`, whose `engine.svelte.ts` consumes the generated `StopwatchState`).
  - `src/components/`: Reusable UI components. `ComingSoon.svelte` consolidates the seven stub route pages; `inspector-pane/` is split into `InspectorPane`, `InspectorTaskFields`/`InspectorEventFields`, and `format.ts`.
- `src-tauri/`: Tauri Rust backend.
  - `src-tauri/src/lib.rs`: Lints, module wiring, `db_pool()`, and `run()`. IPC commands live in `commands/`, not here.
  - `src-tauri/src/commands/`: IPC command handlers split by domain (`tasks.rs`, `events.rs`, `window.rs`, `sync.rs`).
  - `src-tauri/src/error.rs`: `AppError` enum (`thiserror`) returned by ALL IPC commands; serialized as `{code, message}`.
  - `src-tauri/src/events.rs`: Centralized event-name constants (`STOPWATCH_UPDATED`, `SYNC_STARTED/FINISHED/ERROR`, `OAUTH_URL`).
  - `src-tauri/src/db/`: Modularized SQLite operations using `sqlx` (`tasks.rs`, `events.rs`, `settings.rs`, plus `task_filters.rs` for the dynamic `QueryBuilder` filtering path). `init_db` creates tables using inline SQL.
  - `src-tauri/src/domain/`: Core data structures (`mod.rs`, `sigil.rs` for sigil parsing, `filters.rs` for filter columns/types).
  - `src-tauri/src/sync/`: Global sync engine: `mod.rs` (5-minute poller, `SyncState`), `push.rs` (Google push logic), `types.rs`, and the offline queue (`queue.rs`, `queue_store.rs`).
  - `src-tauri/src/stopwatch.rs`: Stopwatch backend (`StopwatchState` struct plus `get/toggle/reset_stopwatch` commands).
  - `src-tauri/src/settings.rs` + `build.rs` + `settings.yaml`: Build-time settings pipeline. `build.rs` parses `settings.yaml` and generates an `AppSettings` struct (template embeds `#[derive(TS)]`) into `OUT_DIR`, which `settings.rs` `include!`s; `ts-rs` also emits `src/lib/bindings/AppSettings.generated.ts`.
  - `src-tauri/src/time_utils/`: Clock strategies, RRULE utilities, and date helpers (`clock_strategies.rs`, `rrule_utils.rs`, `date_utils.rs`).
  - `src-tauri/src/apis/`: 3rd party API integrations (Google Calendar, Google Tasks).
  - `src-tauri/src/auth.rs`: OAuth authentication and token management.
  - `src-tauri/src/screens/`: Screen-specific backend commands and logic (e.g., `plan/`).


## Key Concepts
- **Typed Error Contract**: Every IPC command returns `Result<T, AppError>`. `AppError` serializes as `{code, message}` with kebab-case codes: `db`, `not-found`, `auth`, `sync`, `invalid-input`, `not-ready`, `internal`. The frontend mirror lives in `src/lib/errors.ts` (`BackendErrorCode`). Never return raw strings from commands.
- **Event Name Constants**: Backend event names are constants in `src-tauri/src/events.rs`, mirrored in `src/lib/events.ts` (`stopwatch-updated`, `sync-started`, `sync-finished`, `sync-error`, `oauth-url`). Never inline raw event strings on either side.
- **State Management**: Frontend state is managed using Svelte 5 Runes (`$state`, `$effect`). The primary store is located in `src/lib/store.svelte.ts`, which syncs with the Rust backend via Tauri IPC (`invoke` wrapped in `safeInvoke`). **Crucially, the frontend relies strictly on the SQLite backend as the source of truth.** It avoids complex optimistic patching arrays locally; instead, mutations return `neverthrow` `Result`s, `await` the backend command, and then instantly re-fetch the raw state from the database. This prevents race conditions and UI pop-backs, as local SQLite queries return in ~1-3ms.
- **Generated Bindings Flow**: `cargo test` in `src-tauri` regenerates `src/lib/bindings/*.generated.ts` via `ts-rs` (`export_bindings_*` tests). After changing any `#[derive(TS)]` struct, run `cargo test` and commit the regenerated files. CI fails on binding drift (`git diff --exit-code src/lib/bindings`). The `#[ts(type = "number")]` convention keeps timestamps as JS `number` — never `bigint`.
- **Local-Date Rule**: All day bucketing/comparison must go through `src/lib/time.ts` (`ymd`, `addDays`, `dayDiff`, `sameDay`), which operate on local date parts. `Date.toISOString()` is UTC-shifted and must not be used to derive a calendar day.
- **Database & Migrations**: All tasks and events are stored locally in an SQLite database (`taskroot.db`) located in the app data directory. The schema is created in `db::init_db` using inline SQL queries with `CREATE TABLE IF NOT EXISTS` guards.
- **Deliberate Dead Code**: `get_dirty_tasks`/`get_dirty_events` and some `SyncQueue` methods carry `#[allow(dead_code)]`; they are reserved for the offline-enqueue roadmap (see `TODO.md`). Do not delete them.
- **Multi-Window Architecture**: Three windows are declared in `tauri.conf.json`:
  - **Main Window** (`main`): The primary Svelte app; hides to tray on close.
  - **Launcher Window** (`launcher`): A spotlight-like command palette triggered via the global-shortcut plugin.
  - **Mini Tracker Window** (`minitracker`): A minimal window for tracking time. It runs independently but reads state from the Rust backend.
  All three render the same SPA, branched by window label in `+layout.svelte`.
- **Cross-Window State**: There is no dedicated inter-window wiring module. Window-specific state (stopwatch, sync status) lives in Rust and is synced across windows through Tauri commands (`get_stopwatch_state`, `toggle_stopwatch`, `reset_stopwatch`, `get_sync_state`) plus backend-emitted events consumed via `listen` with the `src/lib/events.ts` constants.

## Style (Important)
- **Svelte 5 Idioms**: Strictly use Svelte 5 runes (`$state`, `$derived`, `$effect`, `$props`) instead of legacy Svelte 4 reactivity (`let foo = ...`, `$:`, `export let`).
- **Rust Idioms**: Write clean, idiomatic Rust. Handle all `Result` and `Option` types safely (do not use `unwrap()` or `expect()` in production code unless absolutely necessary). Use `clippy` for linting.
- **Typescript Idioms**: Strongly type your code. Avoid `any`. Prefer compile-time type inference.
- **Frontend Error Handling (`neverthrow`)**: Use the `neverthrow` library to handle errors functionally on the frontend, mirroring the Rust backend's `Result` type. Do not use standard `try/catch` for expected errors. When calling Tauri's `invoke`, use the `safeInvoke` wrapper (or `useTauriQuery` rune) located in `src/lib/safeInvoke.svelte.ts` to ensure type-safe `ResultAsync` returns.
- **Generated Code**: Never hand-edit anything in `src/lib/bindings/` — regenerate via `cargo test`.
- **Test-Driven Development**: Write tests first as a contract. Do not modify them unless there is something truly wrong.
- **Self-Documenting Code**: Avoid redundant comments. Extract complex logic into well-named functions or constants.
- **Small, Modular Code**: Refactor files if they exceed 250 LOC. Refactor functions with more than 4 levels of indentation.
- **Routing**: Never use magic strings for route paths (e.g., `goto('/login')`). Always import and use the centralized constants from `src/lib/routes.ts` (e.g., `goto(Routes.LOGIN)`).
- Store assets offline.
