# Calendar feature

The CalDAV-backed task and event domain. Base feature: imports no other feature,
`bootstrap`, `ipc`, `platform` or Tauri (its Tauri adapter lives in
`ipc/session.rs`). Outside code should prefer `mod.rs` re-exports.

## Modules

- `domain/` — DTOs, typed IDs, recursive AND/OR `FilterGroup<F>`, recurrence and
  `EventTiming` / `TaskTiming` render normalization.
- `caldav/` — HTTP transport, XML parsers, discovery, multi-profile accounts
  (`account/`) and collection lifecycle (`calendars.rs`).
- `ics/` — byte-level line-preserving `.ics` reader/writer; see
  [CalDAV](../../../../docs/caldav.md) and
  [iCalendar mapping](../../../../docs/icalendar-mapping.md).
- `store/` — in-memory resource map (`model.rs`), recursive query engine
  (`query/`), settings/UI-state JSON (`persist.rs`) and read-modify-write ops
  (`ops/`).
- `load.rs` & `session.rs` — one-shot CalDAV fetch/apply and the headless
  `Session` used by the CLI.
- `occurrences.rs` & `event_query.rs` — recurrence expansion into
  `EventInstance`, and filtered projection that expands matching rows while
  retaining every master/exception as suppression and rule context.
- `window.rs` & `task_dates.rs` — shared `RenderWindow` and task start/due date
  marker projection.

## Projection and filter semantics

- `domain/event_timing.rs` and `domain/task_timing.rs` normalize stored date
  strings for rendering: all-day stays floating; timed resolves in the item's
  zone (UTC fallback).
- `event_query.rs` filters and sorts raw rows, then expands only matching rows
  while keeping all masters/exceptions for cancellation suppression and rule
  inheritance.
- Filters are `FilterGroup<F> { operator, children }` trees with independent
  AND/OR per group:
  - `query/matcher.rs` owns traversal, ignores unfinished leaves and empty
    groups (including inside OR), treats an ineffective root as match-all, ANDs
    text search with the tree, and evaluates multi-value leaves as ANY inclusion
    / NONE exclusion.
  - `is empty` and `is not empty` are effective without a value (stored values
    are ignored). Absence, blank text, empty tag lists, absent priority and
    `TaskPriority::None` all count as empty. Presence and value comparisons live
    in `query/columns.rs`.

## Loading and writes

- `load::fetch_into` / `AccountService::refresh` fetch outside the store lock
  and apply under `Store::revision()`; every resource and collection mutation
  bumps the revision. Startup retries transient `Remote` failures up to 20 times
  (2 s delay); Refresh does not retry.
- `store/ops/` owns conditional writes, writability preflight and scoped
  recurring mutations. See [CalDAV](../../../../docs/caldav.md),
  [known limitations](../../../../docs/known-limitations.md) and
  [settings](../../../../docs/settings.md).
