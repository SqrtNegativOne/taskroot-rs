# Cleanup backlog

In-place work: duplication, dead code and split responsibilities **inside the current
layout**. Nothing here moves a folder or changes a layer, so the module tree stays
recognisable and every step is independently shippable.

The target layout — `features/`, `ipc/`, `platform/`, `bootstrap/`, possibly a separate
crate — lives in [RESTRUCTURE-PLAN.md](RESTRUCTURE-PLAN.md). Finish this list first.

## Backend

1. **Consolidate the IPC surface, and dissolve `screens/plan/mod.rs`.**
   - `caldav/commands.rs` -> `commands/calendars.rs`
   - `settings/mod.rs` commands -> `commands/settings.rs`
   - `screens/plan/mod.rs` query commands -> `commands/plan.rs`; `get_task_schema`/`get_event_schema` -> `commands/schema.rs`
   - `screens/plan/expand.rs` -> `domain/projection.rs`. It is a pure projection, but the CLI and the store tests currently import it from `screens::plan` — a UI module reaching into a headless client.

   Leave the data and settings metadata where they are. Command names do not change; update
   `generate_handler!`. Done when `rg "#\[tauri::command\]" src-tauri/src` matches only
   `commands/` and `rg "screens::plan" src-tauri/src` matches nothing.

2. **Delete `time_utils/`.** It contains one real file. Move `rrule_utils.rs` to
   `domain/timing/recurrence.rs` and remove the folder. Update imports only.

3. **Split `store/ops.rs` by aggregate.** It is ~700 lines with three jobs: the loader,
   event writes (now including `EventScope` resolution) and task writes.
   - `store/ops/load.rs` — `full_load`
   - `store/ops/events.rs` — create/update/delete, scoped recurrence edits, exception writes
   - `store/ops/tasks.rs` — create/update/delete, the recurring-task guard
   - `store/ops/mod.rs` — shared helpers (`resource_href`, `writable_calendar`, `refetch_resource`), re-exported so `ops::delete_event` call sites do not move.

4. **Split the CLI.** `bin/taskroot.rs` is ~850 lines. Move it into `cli/`:
   - `args.rs` — `Cli`, `Verb`, option parsing
   - `session.rs` — data dir, account, `full_load`
   - `commands/{tasks,events,calendars}.rs` — verb bodies
   - `help.rs` — `VERBS`/`OPTIONS`, help text, `docs/cli.md`
   - `output.rs` — JSON and `AppError` output

   `bin/taskroot.rs` becomes a thin `main` calling `tauri_app_lib::cli::run()`.

5. **Split `ics/tests.rs`.** It is ~1100 lines. Split into `tests/{read,write,lines,alarms}.rs`
   plus `fixtures.rs`. No production code changes.

6. **Resolve `DEEP_LINK`.** The backend emits it, but no frontend listener consumes it. Either
   wire the listener or delete the constant and emitter. Do not leave a dead event contract.

7. **Fixtures and mock servers in one place.** `test_support` exists but only holds source
   scans, so setups have drifted into five styles: `ics/fixtures/*.ics`, inline `const
   SERIES_RAW` in `store/tests.rs`, hand-built `AppEvent` literals in `expand.rs` tests, and
   repeated `MockServer::start()` blocks in `caldav/tests/*` and `store/tests.rs`.
   - Rust: `test_support::caldav::start_server()` (started `MockServer` + `test_client`) and `test_support::ics::{series, all_day, with_exception}` built on `ics/fixtures/`. Replace the ~28 repeated mock setups and the inline constants.
   - Frontend: `src/test/factories.ts` with `makeEvent`/`makeInstance`, replacing the `EventInstance` literals in the `bucketing`, `spanLayout`, `dropPayload`, `timing` and `InspectorPane` tests.

8. **Share CalDAV session setup, and fix the password env var.** `bin/taskroot.rs::open()` and
   `commands/session.rs::client_for` both resolve the account, load the password, and build
   `HttpClient`/`CalDavClient`. Extract one `caldav::connect(data_dir, password_override)` helper;
   keep data-dir discovery and load strategy in the adapters. The two paths currently read
   different names: the CLI uses `TASKROOT_CALDAV_PASSWORD`, the app uses
   `TASKROOT_CalDAV_PASSWORD`. Pick `TASKROOT_CALDAV_PASSWORD` and use one constant, so setting
   either cannot silently do nothing.

9. **One recurrence vocabulary.** "Is this a series?" is computed in `store/ops.rs`,
   `screens/plan/expand.rs` and `InspectorPane.svelte`; the scope union exists twice as
   `EventScope` (Rust) and `RecurringMode` (the modal).
   - `AppEvent::is_recurring()` in `domain`, used by `ops` and the projection.
   - Frontend: drop `RecurringMode`; the modal's prop type is `EventScope`, and `src/lib/domain/events.ts` exports `isRecurring(event)` for the inspector.

10. **Split `domain/mod.rs` (510 lines).** Ids, `AppEvent`, `AppTask`, filters and sigils in one
    file. Split into `domain/{ids,event,task,filters}.rs` and re-export from `domain/mod.rs`, so
    imports do not change. `event_timing.rs` stays put; it is the normalization boundary.

11. **Disambiguate the four `events` modules.** Two mean IPC event names, two mean calendar
    events.
    - `src-tauri/src/events.rs` -> `ipc_events.rs`
    - `src/lib/events.ts` -> `src/lib/ipcEvents.ts`
    - `commands/events.rs` and `src/lib/domain/events.ts` keep their names; they really are about calendar events.

12. **Rename the `sync` error code.** With the sync engine gone, `AppError::Sync` means "CalDAV
    or transport failure". Rename it to `remote` in `src-tauri/src/error.rs` and
    `src/lib/errors.ts`, and update the shape tests. Wire-contract change, but the surface is
    small and the current name keeps suggesting a layer that no longer exists.

## Frontend

1. **One snapping implementation.** Create `src/screens/plan/snapping.ts` with `SNAP_MINUTES`,
   `PX_PER_MINUTE`, `MINUTES_PER_DAY`, `snapPixels`, `clampDuration`. Replace:
   - `dropPayload.ts::snappedDropMinute`
   - `EventBlock.svelte::snappedDeltaMinutes`
   - `DayColumn.svelte::snappedMinutesFromOffset`
   - the duration clamping in `eventDrag.svelte.ts` / `taskDrag.svelte.ts`

2. **One local-date/time helper set.** Extend `src/lib/time.ts` with `parseLocalYmd`,
   `formatHhMm`, `MS_PER_MINUTE`. Replace:
   - the two `parseYmd` definitions in `dropPayload.ts` and `spanLayout.ts`
   - `new Date(\`${ymd}T00:00:00\`)` in `PlanScreen.svelte` and `DayColumn.svelte`
   - the six `padStart(2, '0')` clock formatters in `DayCell`, `EventBlock`, `CreationPreview`, `DropPreview`, `CurrentTimeLine`, `TimeGridBackground`

3. **One constant/type home.**
   - Keep `DAYS_IN_WEEK` in `constants.ts`; remove the copy in `spanLayout.ts`.
   - Import `MINUTES_PER_DAY` from `snapping.ts`; remove the copies in `dropPayload.ts`, `bucketing.ts`, and tests.
   - Delete `DayLayoutEvent` in `layout.ts`; let `layoutEvents` accept and return `LaidEvent`.
   - Stop re-exporting constants from `types.ts`; import from `constants.ts` directly.

4. **Delete the stub routes, do not parameterize them.** Remove
   `routes/{docs,graph,launcher,recap,stats,wrap}`, `ComingSoon.svelte`,
   `MoreScreensDropdown.svelte`, their `Routes.*` entries, and the `TitleBar` usage. Dead screens
   do not need a shared title helper.

5. **Unify the drag controllers.** After snapping is extracted, `eventDrag.svelte.ts` and
   `taskDrag.svelte.ts` duplicate the threshold → target tracking → `setDragState` →
   `suppressUpcomingClick` → drop pipeline. Extract `createDropDragController({ dragState,
   payloadFor, onDrop })`; keep only the payload conversion and drop effect per controller.

6. **One recurring-move prompt.** `PlanScreen.svelte` and `day-timeline/DayTimeline.svelte` each
   carry a `PendingReschedule` type, the request/commit/scope handlers and a modal instance —
   the same semantics twice, one of them inside a component the sidebar also mounts. Extract
   `screens/plan/hooks/reschedule.svelte.ts` (`createReschedulePrompt()`, shaped like
   `eventDrag.svelte.ts`) or a `RecurringMovePrompt.svelte` wrapper.

7. **Query definitions in one module.** `query_event_instances` arguments are built separately in
   `DateGrid.svelte` and `DayTimeline.svelte`, and `get_active_calendars` in three components.
   Add `src/lib/queries.ts` exporting named builders (`eventInstances(range, filters)`,
   `allEvents()`, `activeCalendars()`) so command names and argument shapes have one home.
   `commands/tests.rs` keeps guarding the contract.

8. **Give the inspector a state module.** `InspectorPane.svelte` owns three queries, four
   mutation helpers, modal state and scope plumbing. Move them to
   `inspector-pane/state.svelte.ts`, following `screens/sidebar/state.svelte.ts` and
   `screens/do/stopwatch/engine.svelte.ts`.

9. **Use `persistState` in the sidebar.** `screens/sidebar/state.svelte.ts` reimplements
   hydration, validation, debounce and flush for notes/visibility. Use `persistState` for notes
   and show-notes; keep `saveTabTop` as the custom drag-end write.

10. **One debounce primitive.** `useTauriQuery` hand-rolls a timer, pending promise and listener
    lifecycle. Extract the generic debouncer used by `createDebouncedWriter`, then compose it in
    both places. Do not make a query pretend to be a write.

11. **Share the filter/sort menu shell.** `FilterButton.svelte` and `SortButton.svelte` duplicate
    the floating menu, add/remove rows, row styling and outside-click close. Extract
    `FloatingMenu` plus a shared `FilterSortRow`; keep column definitions and operators local.

12. **Share the day-overlap rule.** `bucketEventsByDay` and `instanceOccursOnDay` both decide
    whether a timed instance overlaps a local day. Have bucketing call `instanceOccursOnDay` for
    membership, then add only its minute math.

## Dependencies

Researched 2026-10. Rule: a library must delete more code than the glue it adds, and must
not touch I1–I3.

**Adopt**

- `svelte-sonner` — replaces `Notifications.svelte` (179 lines) and
  `notifications.svelte.ts` (32) with a `<Toaster/>` in the windows that render toasts plus
  `toast.*` calls (4 call sites today). Use `toast.custom` for the click-to-copy case and
  `duration: Infinity` for errors so they stay until dismissed.
- `tempfile` (already in the lockfile) — `tempfile::tempdir()` replaces the three
  hand-rolled `scratch_dir` helpers in `store/tests.rs`, `settings/tests.rs` and
  `caldav/tests/mod.rs`, and stops leaving directories under `target/`.
- `date-fns` (narrow) — the week/day math in `src/lib/time.ts` and its test; keep `ymd`
  hand-written (`format` is the package's heaviest entry point and pulls locale data for a
  fixed ISO pattern). Also fixes a real defect: `src/lib/domain/timing.ts:20` computes the
  day end as `dayStart + 24 h`, so a 25-hour day loses its last hour and a 23-hour day
  leaks the next day's first hour.
- `bits-ui` — start with `Dialog` for `ListManagerModal` and `RecurringActionModal` (both
  bare divs today: no focus trap, no Escape), then `SortButton`/`FilterButton`, which carry
  the same outside-click effect and floating-menu CSS.

**Spike before adopting**

- `ical-rs` 0.6.1 — the only crate that claims byte-for-byte round trips with
  Unknown-preserving properties, which is exactly `ics/lines.rs` (~700 lines plus its
  tests). Three months old, one maintainer, prose-level guarantees. Spike: parse and
  re-serialize `ics/fixtures/*` byte-equal, then one edit through its property lens; repeat
  with a real Radicale resource carrying `ATTENDEE` and `X-*`. `ics/write.rs` stays either
  way — it is merge policy, not line mechanics.
- `libdav` 0.11.1 — covers principal/home discovery, `calendar-query`, `If-Match` PUT,
  `MKCALENDAR` and arbitrary PROPPATCH, so it would replace `caldav/xml.rs` (~400),
  `discovery.rs` (~300) and most of `client.rs` (~190). But it stops at an `HttpClient`
  trait: we would write the hyper adapter ourselves and lose reqwest's TLS roots, proxy and
  redirect policy. Pre-1.0, seven breaking releases in a year.

**Rejected — the reason is the point**

- `tauri-specta` — 2.0 has been an RC for three years and stable 1.0 is Tauri v1 only. It
  would delete the source scans (~325 lines) but needs 114 call sites migrated and a manual
  `impl Type for AppError` (its `Serialize` is hand-written). Revisit at 2.0 final.
- `@tanstack/svelte-query` — wants thrown `Error`s; every `queryFn` becomes a wrapper that
  loses `Result` typing, and the cache buys nothing when every mutation invalidates
  everything.
- `jiff` — no 1.0 and no stated timeline, and `rrule`/`icalendar` hand us chrono types
  anyway, so chrono stays a dependency regardless.
- `insta` — snapshots pin whole `.ics` bodies and turn the invariant tests into change
  detectors exactly where the writer intentionally preserves unknown properties. Only the
  four `create_*_resource` tests would gain.
- `clap` — the `VERBS`/`OPTIONS` tables are also the source of `docs/cli.md`, so the real
  saving is ~120–150 lines, not ~230, and the doc wording is a product decision.
- `es-toolkit`, `svelte-dnd-action`, `@atlaskit/pragmatic-drag-and-drop`, `zod`/`valibot`,
  `virtua`, `runed`, `schemars`, `backon` — each adds a dependency and a concept for ≤50
  lines, or does not model a 2-D time grid.

**Never substitute**

- `ics/lines.rs` + `ics/write.rs`: `icalendar` keeps properties and parameters in maps, so
  any parse→serialize round trip reorders and re-folds the whole resource, and there is no
  raw/verbatim mode. It stays the reader and the new-resource builder.
- `rrule_utils.rs`: `rrule`'s `Display` always emits `UNTIL=…T…Z`, so a date-valued all-day
  `UNTIL` cannot be produced, and it canonicalizes part order — round-tripping a rule
  rewrites parts we never touched.
- The timeline drag/snapping stack: no library hit-tests a 2-D time grid, snaps to minutes
  and then prompts for a recurrence scope.

## Docs

1. **One home per fact.** The `AGENTS.md` files are rules, `docs/caldav.md` is protocol, this file
   is the in-place backlog, `RESTRUCTURE-PLAN.md` is the target layout. Link instead of
   restating: adding the scoped-delete API touched four documents. When something changes,
   update the one that owns the fact and let the others point at it.

## Deferred

Do not do these as part of a cleanup pass:

- A generic `packLanes` for `layoutEvents` and `assignLanes`. The algorithms solve different axes; merge only when both are being changed for another reason.
- A shared Rust wall-clock/date-anchor helper across `event_timing`, `rrule_utils`, `reminders`, and `ics::write`. That needs a real design, not a mechanical extraction.
- Any `core/` / `app/`, `features/`, `ipc/` or crate split — see [RESTRUCTURE-PLAN.md](RESTRUCTURE-PLAN.md).

## After each step

```bash
bun run check
bun run lint
cargo clippy --all-targets -- -D warnings
cargo nextest run
```

Run `bun run test:unit` when the step touches frontend code. `cargo nextest run` also regenerates
bindings and `docs/cli.md`.
