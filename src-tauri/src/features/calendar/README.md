# Calendar feature

The CalDAV-backed task and event domain. It is the base feature: it imports no
other feature, `bootstrap`, `ipc`, `platform`, or Tauri.

## Modules

- `domain/` — DTOs, ids, filters, recurrence and the `EventTiming` normalization
  boundary.
- `caldav/` — HTTP transport, XML, discovery, account setup and resource CRUD.
- `ics/` — byte-level `.ics` read/write. Never serialize a DTO into a resource.
- `store/` — the in-memory resource map, query path, settings/UI-state JSON and
  read-modify-write operations.
- `load.rs` — pure one-shot load from CalDAV into a store.
- `session.rs` — the headless `Session` (store + client + data dir) and the
  collection projection used by IPC.
- `occurrences.rs` — expands stored events into render-ready `EventInstance`s.

`mod.rs` is the only façade. Submodules are private to the feature; outside code
uses the re-exports there. The Tauri adapter lives in `ipc/session.rs`, not here.
