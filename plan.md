# Architecture & Simplification Plan

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

## Phase 4 — Evaluate later (only if pain grows)

- [ ] **CalDAV client (`libdav`)** — consider replacing ~1,000 LOC in `caldav/{xml,discovery,transport,client}.rs` if targeting servers beyond Radicale (Fastmail, Nextcloud, iCloud, Baïkal).
- [ ] **Optional frontend ergonomics** `chrono-node` (TS) if quick-capture needs richer natural-language dates; JS `rrule` `toText()` if recurrence UX expands beyond the 6 presets.

## Library & Language Decision Notes

- **Why stay in Rust for ICS/CalDAV?**
  - Keeps one shared headless core for the GUI, `taskroot-cli`, and background reminders without a full rewrite.
  - **TypeScript (`ical.js` + `tsdav`)**: `ical.js` is best-in-class for round-tripping, but moving the backend/CLI to TS + `@tauri-apps/plugin-http` is a full rewrite with more short-term risk than swapping Rust crates.
  - **Go sidecar (`go-ical` + `go-webdav`)**: clean libraries, but adds a 3rd language, sidecar IPC, and cross-compile overhead.
  - **C (`libical` FFI) / Python / JVM**: Windows MSVC/CMake FFI pain (`libical`) or heavy runtime packaging (Python/JVM).
