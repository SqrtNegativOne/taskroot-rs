# Architecture & Simplification Plan

## Phase 1 — Quick wins (low risk, immediate cleanup)

- [ ] **Remove unused / stray dependencies**
  - Drop unused `tauri-plugin-store` from [src-tauri/Cargo.toml](src-tauri/Cargo.toml) and [src-tauri/src/bootstrap/mod.rs](src-tauri/src/bootstrap/mod.rs).
  - Replace the single `color-eyre` usage in [recurrence.rs](src-tauri/src/features/calendar/domain/timing/recurrence.rs) with `AppError` and drop `color-eyre`.
- [ ] **Make local JSON persistence atomic**
  - Move `tempfile` from `[dev-dependencies]` to `[dependencies]` and write temp-file + rename in [persist.rs](src-tauri/src/features/calendar/store/persist.rs) so crashes cannot truncate `settings.json`, `ui-state.json`, or `account.json`.
- [ ] **Split oversized files & inline tests**
  - Split inline tests out of [stopwatch/state.rs](src-tauri/src/features/stopwatch/state.rs) (843 LOC) into a dedicated test module.
  - Remove legacy `X-TASKROOT-*` stripping lines from [ics/write.rs](src-tauri/src/features/calendar/ics/write.rs) (per the no-legacy-compatibility rule) and fix the stale retry comment in [store.svelte.ts](src/lib/store.svelte.ts).

---

## Phase 2 — Fix real write-path & concurrency bugs

- [ ] **Switch inspector edits from full-DTO `PUT` to field patches**
  - Replace full-DTO `update_task` / `update_event` with flat tagged patches (`TaskPatch` / `EventPatch`, e.g. `Title(String)`, `Due(Option<String>)`, `Notes(Option<String>)`).
  - Eliminates "field absent" vs. "field cleared" ambiguity.
  - Fixes rapid inspector edits clobbering each other in [inspector-pane/state.svelte.ts](src/components/inspector-pane/state.svelte.ts).
  - Fixes `412 Conflict` retries in [ops/tasks.rs](src-tauri/src/features/calendar/store/ops/tasks.rs) and [ops/events.rs](src-tauri/src/features/calendar/store/ops/events.rs) silently overwriting concurrent edits from tasks.org/phone (refetch → re-apply patch only → `PUT`).
- [ ] **Stop holding `Mutex<Store>` across network I/O**
  - Snapshot inputs under the lock, release `StoreState` during HTTP `PUT`/`DELETE`/refetch in [ipc/tasks.rs](src-tauri/src/ipc/tasks.rs) and [ipc/events.rs](src-tauri/src/ipc/events.rs), then re-acquire and apply with revision/ETag checks.
  - Prevents slow/stalled CalDAV writes from blocking queries, settings, and [reminders/tauri.rs](src-tauri/src/features/reminders/tauri.rs).
- [ ] **Fix UTC `VEVENT` round-trip bug**
  - Preserve `UTC` on `DTSTART:...Z` events in [ics/read.rs](src-tauri/src/features/calendar/ics/read.rs) / [ics/write.rs](src-tauri/src/features/calendar/ics/write.rs) (currently `timezone = None`, causing edits to rewrite UTC events as floating local time).
- [ ] **Clean up `STORE_UPDATED` fan-out**
  - Emit `STORE_UPDATED` solely from Rust (not split between [store.svelte.ts](src/lib/store.svelte.ts) and Rust IPC).
  - Stop re-fetching `settings.json` on every calendar mutation, and remove duplicate `get_active_calendars` queries (e.g. [inspector-pane/state.svelte.ts](src/components/inspector-pane/state.svelte.ts) vs. [InspectorTaskFields.svelte](src/components/inspector-pane/InspectorTaskFields.svelte)).

---

## Phase 3 — Replace high-maintenance hand-rolled subsystems

- [ ] **Spike & replace custom ICS line editor (~2,100 LOC)**
  - Why: byte-identical folding/order is unnecessary — CalDAV only needs **semantic lossless round-tripping** (preserving unknown properties, parameters, `VTIMEZONE`, unmodelled `VALARM`s, and sibling components). Current `icalendar` `0.17` breaks round-tripping; custom [ics/lines.rs](src-tauri/src/features/calendar/ics/lines.rs) has its own RFC 5545 edge cases (case-sensitive matching, naive `split_once(':')` on quoted params, dual-parser `nth_kind` index coupling).
  - Spike **`calcard-next`** (community fork of Stalwart's `calcard`) and **`ical-rs`** behind `ics/` against the existing Mozilla + tasks.org fixtures in `ics/tests/`.
  - Relax assertions from byte-identical folding/order to semantic preservation; delete `ics/lines.rs`, the hand-rolled ISO-8601 duration parser, and `icalendar` once green.
- [ ] **Migrate headless CLI to `clap`**
  - Replace hand-rolled [cli/args.rs](src-tauri/src/cli/args.rs), [cli/help.rs](src-tauri/src/cli/help.rs), and manual option parsing in `cli/commands/` with `clap` (`derive`) + generated `docs/cli.md`.
  - Fixes silent unknown-flag typos, missing `--key=value` / `--` handling, and doc/code drift (e.g. `task-show <id>...` currently ignoring extra IDs).
- [ ] **Replace `ts-rs` + IPC text scanner with `tauri-specta` v2**
  - Generate typed frontend `commands.*` invoke wrappers and event types directly from Rust.
  - Delete ~400 LOC of heuristic TS scanning in [ipc/tests.rs](src-tauri/src/ipc/tests.rs), [frontend_scan.rs](src-tauri/src/test_support/frontend_scan.rs), and [source_scan.rs](src-tauri/src/test_support/source_scan.rs).

---

## Phase 4 — Evaluate later (only if pain grows)

- [ ] **CalDAV client (`libdav`)** — consider replacing ~1,000 LOC in `caldav/{xml,discovery,transport,client}.rs` if targeting servers beyond Radicale (Fastmail, Nextcloud, iCloud, Baïkal).
- [ ] **Simplify [taskroot-macros](src-tauri/taskroot-macros/src/lib.rs) (418 LOC)** — evaluate replacing the custom `syn` proc-macro crate (used by only 3 structs: `AppTask`, `AppEvent`, `AppSettings`) with plain Rust tables or small `macro_rules!`.
- [ ] **Typed dates/times across domain DTOs** — consolidate duplicated `ICAL_DATE`/`DTO_DATE`/`wall_clock`/`is_all_day` parsing across `ics/`, `event_timing.rs`, and `recurrence.rs`.
- [ ] **Optional frontend ergonomics** — `paneforge` for [SplitPane.svelte](src/components/SplitPane.svelte) if resize edge cases appear; `chrono-node` (TS) if quick-capture needs richer natural-language dates; JS `rrule` `toText()` if recurrence UX expands beyond the 6 presets.

---

## Library & Language Decision Notes

- **Why stay in Rust for ICS/CalDAV?**
  - Keeps one shared headless core for the GUI, `taskroot-cli`, and background reminders without a full rewrite.
  - **TypeScript (`ical.js` + `tsdav`)**: `ical.js` is best-in-class for round-tripping, but moving the backend/CLI to TS + `@tauri-apps/plugin-http` is a full rewrite with more short-term risk than swapping Rust crates.
  - **Go sidecar (`go-ical` + `go-webdav`)**: clean libraries, but adds a 3rd language, sidecar IPC, and cross-compile overhead.
  - **C (`libical` FFI) / Python / JVM**: Windows MSVC/CMake FFI pain (`libical`) or heavy runtime packaging (Python/JVM).
