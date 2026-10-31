# Cleanup backlog

The target layout — `features/`, `ipc/`, `platform/`, `bootstrap/` — lives in [RESTRUCTURE-PLAN.md](RESTRUCTURE-PLAN.md). Finish this list first.

## Backend

1. **Consolidate the IPC surface, and dissolve `screens/plan/mod.rs`.**
   - `caldav/commands.rs` -> `commands/calendars.rs`
   - `settings/mod.rs` commands -> `commands/settings.rs`
   - `screens/plan/mod.rs` query commands -> `commands/plan.rs`; `get_task_schema`/`get_event_schema` -> `commands/schema.rs`
   - `screens/plan/expand.rs` -> `domain/projection.rs`. It is a pure projection, but the CLI and the store tests currently import it from `screens::plan` — a UI module reaching into a headless client.

   Leave the data and settings metadata where they are. Command names do not change; update
   `generate_handler!`. Done when `rg "#\[tauri::command\]" src-tauri/src` matches only
   `commands/` and `rg "screens::plan" src-tauri/src` matches nothing. **✅ done**

3. **Split `store/ops.rs` by aggregate.** It is ~700 lines with three jobs: the loader,
   event writes (now including `EventScope` resolution) and task writes.
   - `store/ops/load.rs` — `full_load`
   - `store/ops/events.rs` — create/update/delete, scoped recurrence edits, exception writes
   - `store/ops/tasks.rs` — create/update/delete, the recurring-task guard
   - `store/ops/mod.rs` — shared helpers (`resource_href`, `writable_calendar`, `refetch_resource`), re-exported so `ops::delete_event` call sites do not move.
   **✅ done**

4. **Split the CLI.** `bin/taskroot.rs` is ~850 lines. Move it into `cli/`:
   - `args.rs` — `Cli`, `Verb`, option parsing
   - `session.rs` — data dir, account, `full_load`
   - `commands/{tasks,events,calendars}.rs` — verb bodies
   - `help.rs` — `VERBS`/`OPTIONS`, help text, `docs/cli.md`
   - `output.rs` — JSON and `AppError` output

   `bin/taskroot.rs` becomes a thin `main` calling `tauri_app_lib::cli::run()`. **✅ done**

7. **Fixtures and mock servers in one place.** `test_support` exists but only holds source
   scans, so setups have drifted into five styles: `ics/fixtures/*.ics`, inline `const
   SERIES_RAW` in `store/tests.rs`, hand-built `AppEvent` literals in `expand.rs` tests, and
   repeated `MockServer::start()` blocks in `caldav/tests/*` and `store/tests.rs`.
   - Rust: `test_support::caldav::start_server()` (started `MockServer` + `test_client`) and `test_support::ics::{series, all_day, with_exception}` built on `ics/fixtures/`. Replace the ~28 repeated mock setups and the inline constants.
   - Frontend: `src/test/factories.ts` with `makeEvent`/`makeInstance`, replacing the `EventInstance` literals in the `bucketing`, `spanLayout`, `dropPayload`, `timing` and `InspectorPane` tests.
   **✅ done**

8. **Share CalDAV session setup, and fix the password env var.** `bin/taskroot.rs::open()` and
   `commands/session.rs::client_for` both resolve the account, load the password, and build
   `HttpClient`/`CalDavClient`. Extract one `caldav::connect(data_dir, password_override)` helper;
   keep data-dir discovery and load strategy in the adapters. The two paths currently read
   different names: the CLI uses `TASKROOT_CALDAV_PASSWORD`, the app uses
   `TASKROOT_CalDAV_PASSWORD`. Pick `TASKROOT_CALDAV_PASSWORD` and use one constant, so setting
   either cannot silently do nothing. **✅ done**

9. **One recurrence vocabulary.** "Is this a series?" is computed in `store/ops.rs`,
   `screens/plan/expand.rs` and `InspectorPane.svelte`; the scope union exists twice as
   `EventScope` (Rust) and `RecurringMode` (the modal).
   - `AppEvent::is_recurring()` in `domain`, used by `ops` and the projection.
   - Frontend: drop `RecurringMode`; the modal's prop type is `EventScope`, and `src/lib/domain/events.ts` exports `isRecurring(event)` for the inspector. **✅ done**

## Frontend

2. **One local-date/time helper set. ✅ done** Extend `src/lib/time.ts` with `parseLocalYmd`,
   `formatHhMm`, `MS_PER_MINUTE`. Replace:
   - the two `parseYmd` definitions in `dropPayload.ts` and `spanLayout.ts`
   - `new Date(\`${ymd}T00:00:00\`)` in `PlanScreen.svelte` and `DayColumn.svelte`
   - the six `padStart(2, '0')` clock formatters in `DayCell`, `EventBlock`, `CreationPreview`, `DropPreview`, `CurrentTimeLine`, `TimeGridBackground` **✅ done (fc7bbd7)** (all unified to zero-padded `HH:MM`)

4. ~~**Delete the stub routes, do not parameterize them.**~~ **Dropped — keep the stub routes**
   (decided 2026-07-24; `routes/{docs,graph,launcher,recap,stats,wrap}`, `ComingSoon.svelte`,
   `MoreScreensDropdown.svelte`, their `Routes.*` entries and the `TitleBar` usage all stay).

5. **Unify the drag controllers.** After snapping is extracted, `eventDrag.svelte.ts` and
   `taskDrag.svelte.ts` duplicate the threshold → target tracking → `setDragState` →
   `suppressUpcomingClick` → drop pipeline. Extract `createDropDragController({ dragState,
   payloadFor, onDrop })`; keep only the payload conversion and drop effect per controller.
   **✅ done**

6. **One recurring-move prompt.** `PlanScreen.svelte` and `day-timeline/DayTimeline.svelte` each
   carry a `PendingReschedule` type, the request/commit/scope handlers and a modal instance —
   the same semantics twice, one of them inside a component the sidebar also mounts. Extract
   `screens/plan/hooks/reschedule.svelte.ts` (`createReschedulePrompt()`, shaped like
   `eventDrag.svelte.ts`) or a `RecurringMovePrompt.svelte` wrapper. **✅ done**
   
8. **Give the inspector a state module.** **✅ done** `InspectorPane.svelte` owned three
   queries, four mutation helpers, modal state and scope plumbing; they now live in
   `inspector-pane/state.svelte.ts`, following `screens/sidebar/state.svelte.ts` and
   `screens/do/stopwatch/engine.svelte.ts`.

9. **Use `persistState` in the sidebar.** `screens/sidebar/state.svelte.ts` reimplements
   hydration, validation, debounce and flush for notes/visibility. Use `persistState` for notes
   and show-notes; keep `saveTabTop` as the custom drag-end write. **✅ done**

10. **One debounce primitive.** `useTauriQuery` hand-rolls a timer, pending promise and listener
    lifecycle. Extract the generic debouncer used by `createDebouncedWriter`, then compose it in
    both places. Do not make a query pretend to be a write. **✅ done**

11. **Share the filter/sort menu shell.** `FilterButton.svelte` and `SortButton.svelte` duplicate
    the floating menu, add/remove rows, row styling and outside-click close. Extract
    `FloatingMenu` plus a shared `FilterSortRow`; keep column definitions and operators local.
    **✅ done**

## Dependencies

Researched 2026-10. Rule: a library must delete more code than the glue it adds.

**Adopt**

- `svelte-sonner` — **✅ done** — replaces `Notifications.svelte` (179 lines) and
  `notifications.svelte.ts` (32) with a `<Toaster/>` in the windows that render toasts plus
  `toast.*` calls (4 call sites today). Use `toast.custom` for the click-to-copy case and
  `duration: Infinity` for errors so they stay until dismissed.
- `date-fns` (narrow) — **✅ done** — the week/day math in `src/lib/time.ts` and its test; keep `ymd`
  hand-written (`format` is the package's heaviest entry point and pulls locale data for a
  fixed ISO pattern). Also fixes a real defect: `src/lib/domain/timing.ts:20` computes the
  day end as `dayStart + 24 h`, so a 25-hour day loses its last hour and a 23-hour day
  leaks the next day's first hour.
- `bits-ui` — **✅ done** — `Dialog` for `ListManagerModal` and `RecurringActionModal`
  (focus-trapped, Escape/overlay-closable) and `Popover` for the shared `SortButton`/
  `FilterButton` `FloatingMenu`, replacing its outside-click effect and floating-menu CSS.

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

## After each step

```bash
bun run check
bun run lint
cargo clippy --all-targets -- -D warnings
cargo nextest run
```

Run `bun run test:unit` when the step touches frontend code. `cargo nextest run` also regenerates
bindings and `docs/cli.md`.
