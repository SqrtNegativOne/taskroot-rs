# Calendar feature

The CalDAV-backed task and event domain. It is the base feature: it imports no
other feature, `bootstrap`, `ipc`, `platform`, or Tauri.

## Modules

- `domain/` — DTOs, ids, typed recursive AND/OR filter groups, recurrence and the
  `EventTiming` normalization boundary.
- `caldav/` — HTTP transport, XML, discovery, saved-account profiles, switching
  (`account/`) and resource CRUD.
- `ics/` — byte-level `.ics` read/write. Never serialize a DTO into a resource.
- `store/` — the in-memory resource map, query path (`query/matcher.rs` for
  recursive filters, `columns.rs` for comparisons, `sorting.rs` for order),
  settings/UI-state JSON and read-modify-write operations.
- `load.rs` — pure one-shot load from CalDAV into a store.
- `session.rs` — the headless `Session` (store + client + data dir) and the
  collection projection used by IPC.
- `occurrences.rs` — expands stored events into render-ready `EventInstance`s;
  `occurrences/identity.rs` supplies occurrence/exclusion identity helpers.
- `event_query.rs` — shared IPC/CLI filtered projection: expand only sorted
  matching rows, but keep every master/exception as suppression/rule context.

`mod.rs` is the public entry point; outside code should prefer its re-exports.
Several submodules are currently `pub(crate)`, so this is a convention rather than
Rust-enforced privacy. The Tauri adapter lives in `ipc/session.rs`, not here.

## Projection and filters

`domain/event_timing.rs` and `domain/task_timing.rs` normalize stored date strings
for rendering. All-day values stay floating; timed values resolve in the item's
zone (UTC fallback). `window.rs` supplies the shared local-date/instant render
window. `task_dates.rs` projects each task start/due separately.

`event_query.rs` selects and sorts matching raw rows, then expands only those
rows while retaining all masters/exceptions for suppression and rule inheritance.
Filters and search must not discard recurrence context before expansion.

Filters are `FilterGroup<F> { operator, children }` trees with tagged condition
and group nodes. Each group chooses AND/OR independently. `query/matcher.rs`
ignores unfinished leaves and groups without effective children, including inside
OR; an ineffective root matches all. Search is ANDed with the tree. Leaf
multi-values retain ANY inclusion / NONE exclusion semantics. `is empty` and
`is not empty` are effective conditions without a value; any stored value is
ignored. Absence, blank text and empty tag lists count as empty. An absent priority
and explicit `TaskPriority::None` both count as empty. Presence matching lives
beside value comparisons in `query/columns.rs`; `matcher.rs` owns tree traversal,
unfinished-condition handling and multi-value inclusion/exclusion.

## Loading and writes

`load::fetch_into` re-reads the client's account metadata, refreshes discovery and
fetches every
collection without touching a store; `apply` swaps a complete snapshot in.
`ipc::session::load_all` applies under a revision guard after fetching outside the
store lock. Collection-list refresh uses a three-second timeout and falls back
to the stored list on failure; full account discovery uses the shared request
timeout instead. Startup retries remote failures up to 20 attempts with
two-second delays (request time is additional); Refresh does not run that retry
loop. A stale snapshot is skipped if the store changed meanwhile.

`store/ops` owns read-modify-write, conditional requests and writable-list
selection. Ordinary same-collection updates get one conflict re-fetch/retry;
moves and deletes currently propagate conflicts. See [CalDAV](../../../../docs/caldav.md)
for I1–I3, property mappings, nested-alarm preservation and scoped edits, and
[known limitations](../../../../docs/known-limitations.md) for source-writability,
revision, mixed-alarm and exception-rescheduling gaps. `following`
deletes truncate the series; `following` moves and recurring-task completion are
intentionally unsupported until their series semantics are implemented.

Settings/UI-state persistence lives in `store/persist.rs`; see
[settings behavior](../../../../docs/architecture.md#adding-a-setting).
