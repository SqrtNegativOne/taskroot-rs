# Cleanup backlog

Architecture work is deferred. This is a list of concrete duplication, dead code, and split responsibilities. Do not start a `core/` / `app/` split, a `screens/` -> `features/` rename, or a separate crate from this file.

## Backend

1. **Consolidate the IPC surface.** Move the handlers currently living in domain modules into `commands/`:
   - `caldav/commands.rs` -> `commands/calendars.rs`
   - `settings/mod.rs` commands -> `commands/settings.rs`
   - `screens/plan/mod.rs` commands -> `commands/plan.rs`

   Leave the data, projection and settings metadata where they are. Command names do not change; update `generate_handler!`. Done when `rg "#\[tauri::command\]" src-tauri/src` matches only `commands/`.

2. **Delete `time_utils/`.** It contains one file. Move `rrule_utils.rs` to `domain/timing/recurrence.rs` and remove the folder. Update imports only.

3. **Split `store/ops.rs` by direction.** It currently owns both reading and writing:
   - `store/ops/load.rs`: `full_load`, `pull`, snapshot fallback
   - `store/ops/write.rs`: create/update/delete, `PUT`/`DELETE`, `412` retry
   - `store/ops/mod.rs`: shared helpers only

   Move the collection diffing out of `sync/mod.rs::sync_all` into `load.rs`; scheduling and events stay in `sync/`. No behaviour change.

4. **Split the CLI.** `bin/taskroot.rs` is ~800 lines. Move it into `cli/`:
   - `args.rs` — `Cli`, `Verb`, option parsing
   - `session.rs` — data dir, account, `full_load`
   - `commands/{tasks,events,calendars}.rs` — verb bodies
   - `help.rs` — `VERBS`/`OPTIONS`, help text, `docs/cli.md`
   - `output.rs` — JSON and `AppError` output

   `bin/taskroot.rs` becomes a thin `main` calling `tauri_app_lib::cli::run()`.

5. **Split `ics/tests.rs`.** It is ~1100 lines. Split into `tests/{read,write,lines,alarms}.rs` plus `fixtures.rs`. No production code changes.

6. **Resolve `DEEP_LINK`.** The backend emits it, but no frontend listener consumes it. Either wire the listener or delete the constant and emitter. Do not leave a dead event contract.

7. **Add a CalDAV test harness.** `MockServer::start()` is repeated ~28 times across `caldav/tests/{client,transport,discovery}.rs` and `store/tests.rs`. Add `start_server()` plus response fixtures to `caldav/tests/mod.rs`, next to `test_client`.

8. **Share CalDAV session setup, and fix the password env var.** `bin/taskroot.rs::open()` and `commands/session.rs::client_for` both resolve the account, load the password, and build `HttpClient`/`CalDavClient`. Extract one `caldav::connect(data_dir, password_override)` helper; keep data-dir discovery and load strategy in the adapters. The two paths currently read different names: the CLI uses `TASKROOT_CALDAV_PASSWORD`, the app uses `TASKROOT_CalDAV_PASSWORD`. Pick `TASKROOT_CALDAV_PASSWORD` and use one constant, so setting either cannot silently do nothing.

   Do not share the CLI's per-invocation `full_load` with the app's poller/snapshot path; those are deliberately different.

## Frontend

1. **One snapping implementation.** Create `src/screens/plan/snapping.ts` with `SNAP_MINUTES`, `PX_PER_MINUTE`, `MINUTES_PER_DAY`, `snapPixels`, `clampDuration`. Replace:
   - `dropPayload.ts::snappedDropMinute`
   - `EventBlock.svelte::snappedDeltaMinutes`
   - `DayColumn.svelte::snappedMinutesFromOffset`
   - the duration clamping in `eventDrag.svelte.ts` / `taskDrag.svelte.ts`

2. **One local-date/time helper set.** Extend `src/lib/time.ts` with `parseLocalYmd`, `formatHhMm`, `MS_PER_MINUTE`. Replace:
   - the two `parseYmd` definitions in `dropPayload.ts` and `spanLayout.ts`
   - `new Date(\`${ymd}T00:00:00\`)` in `PlanScreen.svelte` and `DayColumn.svelte`
   - the six `padStart(2, '0')` clock formatters in `DayCell`, `EventBlock`, `CreationPreview`, `DropPreview`, `CurrentTimeLine`, `TimeGridBackground`

3. **One constant/type home.**
   - Keep `DAYS_IN_WEEK` in `constants.ts`; remove the copy in `spanLayout.ts`.
   - Import `MINUTES_PER_DAY` from `snapping.ts`; remove the copies in `dropPayload.ts`, `bucketing.ts`, and tests.
   - Delete `DayLayoutEvent` in `layout.ts`; let `layoutEvents` accept and return `LaidEvent`.
   - Stop re-exporting constants from `types.ts`; import from `constants.ts` directly.

4. **Delete the stub routes, do not parameterize them.** Remove `routes/{docs,graph,launcher,recap,stats,wrap}`, `ComingSoon.svelte`, `MoreScreensDropdown.svelte`, their `Routes.*` entries, and the `TitleBar` usage. Dead screens do not need a shared title helper. Keep `routes/dev`.

5. **Clean up the dev page.** `handlePush` and `handlePull` are identical; replace both with one “Sync now” button. Remove the localStorage inspector section: it inspects a channel the app no longer uses. Keep sync state and the wipe action.

6. **Unify the drag controllers.** After snapping is extracted, `eventDrag.svelte.ts` and `taskDrag.svelte.ts` duplicate the threshold → target tracking → `setDragState` → `suppressUpcomingClick` → drop pipeline. Extract `createDropDragController({ dragState, payloadFor, onDrop })`; keep only the payload conversion and drop effect per controller.

7. **Use `persistState` in the sidebar.** `screens/sidebar/state.svelte.ts` reimplements hydration, validation, debounce and flush for notes/visibility. Use `persistState` for notes and show-notes; keep `saveTabTop` as the custom drag-end write.

8. **One debounce primitive.** `useTauriQuery` hand-rolls a timer, pending promise and listener lifecycle. Extract the generic debouncer used by `createDebouncedWriter`, then compose it in both places. Do not make a query pretend to be a write.

9. **Share the filter/sort menu shell.** `FilterButton.svelte` and `SortButton.svelte` duplicate the floating menu, add/remove rows, row styling and outside-click close. Extract `FloatingMenu` plus a shared `FilterSortRow`; keep column definitions and operators local.

10. **Share the day-overlap rule.** `bucketEventsByDay` and `instanceOccursOnDay` both decide whether a timed instance overlaps a local day. Have bucketing call `instanceOccursOnDay` for membership, then add only its minute math.

## Deferred architecture

Do not do these as part of this pass:

- A generic `packLanes` for `layoutEvents` and `assignLanes`. The algorithms solve different axes; merge only when both are being changed for another reason.
- A shared Rust wall-clock/date-anchor helper across `event_timing`, `rrule_utils`, `reminders`, and `ics::write`. That needs a real design, not a mechanical extraction.
- Any `core/` / `app/`, `features/`, or crate split.

## After each step

```bash
bun run check
bun run lint
cargo clippy --all-targets -- -D warnings
cargo nextest run
```

Run `bun run test:unit` when the step touches frontend code. `cargo nextest run` also regenerates bindings and `docs/cli.md`.
