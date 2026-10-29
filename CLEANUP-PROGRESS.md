# Cleanup backlog — progress (handoff)

Status as of this note. Five backend items are committed and fully verified; the
frontend batch is planned in detail below but **not started** (a few call sites are
read and analyzed, nothing written).

## Done (committed on `main`, full backend suite green: clippy `-D warnings` + 250/250 nextest)

| Item | Commit | Notes |
| --- | --- | --- |
| Backend 2 — delete `time_utils/` | `afc6c5c` | `rrule_utils.rs` → `src-tauri/src/domain/timing/recurrence.rs`; `timing/mod.rs` does `pub mod recurrence; pub use recurrence::*;`; `domain/mod.rs` re-exports `timing::*`. Imports updated in `screens/plan/expand.rs` and `store/ops.rs`. |
| Backend 11 — rename events modules | `9070290` | `src-tauri/src/events.rs` → `ipc_events.rs` (callers: `lib.rs`, `commands/stopwatch.rs`, `load.rs`); `src/lib/events.ts` → `ipcEvents.ts` (callers: `safeInvoke.svelte.ts`, `store.svelte.ts`, `routes/effect/+page.svelte`, `routes/settings/+page.svelte`, `stopwatch/engine.svelte.ts`). |
| Backend 10 — split `domain/mod.rs` | `90ffed9` | Now `domain/{ids,task,event}.rs` + re-exports in `mod.rs` (imports unchanged). Judgment calls made: `WindowLabel` went to `ids.rs` (identity enums); `AppCalendar` went to `event.rs` (no `calendar.rs` was specified). `filters.rs`/`sigil.rs`/`event_timing.rs` untouched. One binding diff: an intra-doc link that no longer resolved in the new module was made plain text, so `AppTask.generated.ts` lost the `[ ]` around `AppEvent` in a comment (CI-safe, committed). |
| Backend 5 — split `ics/tests.rs` | `b96e631` | Now `ics/tests/{mod,fixtures,read,write,lines,alarms}.rs`; 48/48 ics tests pass. `fixtures.rs` = fixture constants + shared helpers (`pub(super)`). Test placement: pure reads → `read.rs`; all merge/delete/create/insert + non-alarm VTODO fidelity → `write.rs`; escaping/folding/surface-preservation (incl. the proptest) → `lines.rs`; everything VALARM → `alarms.rs`. No production code touched. |
| Backend 12 — `sync` → `remote` | `46b4aa2` | `AppError::Remote` in `error.rs` + code `"remote"`; call sites in `caldav/{discovery,transport,xml}.rs`; frontend `errors.ts` + `store.test.ts` + `PlanScreen.test.ts`; `docs/caldav.md` status-mapping line. **Wire-contract change** — both sides moved together, but flagging it per your "decisions to me" rule. |

## Not started — planned, ready to execute

1. **Frontend 1 — `src/screens/plan/snapping.ts`.** Owns `SNAP_MINUTES` (15), `PX_PER_MINUTE`, `MINUTES_PER_DAY`, plus:
   - `snapPixels(px)` = `Math.round(px / PX_PER_MINUTE / SNAP_MINUTES) * SNAP_MINUTES` — replaces `snappedDropMinute`'s snap core (`dropPayload.ts`), `snappedDeltaMinutes` (`EventBlock.svelte`), `snappedMinutesFromOffset` (`DayColumn.svelte`).
   - `clampDuration(mins)` = `Math.min(Math.max(mins, SNAP_MINUTES), MINUTES_PER_DAY)` — replaces the clamping in `eventDrag.svelte.ts` / `taskDrag.svelte.ts` (read those two next).
   - Read `PX_PER_MIN`/`SNAP_MIN` from `day-timeline/constants.ts`; keep the pixel constants there (only move the minute-domain constants).
2. **Frontend 2 — extend `src/lib/time.ts`** (its tests in `time.test.ts`):
   - `parseLocalYmd(value: string): Date` = `new Date(`${value}T00:00:00`)` — replaces the two `parseYmd` copies in `dropPayload.ts`/`spanLayout.ts`, the `` new Date(`${…}T00:00:00`) `` in `PlanScreen.svelte` and `DayColumn.svelte` (×2, in `handleResize`/`handleMove`).
   - `formatHhMm(minute: number): string` — zero-padded 24h clock from a minutes value; replaces the six local clock formatters (`pad2`/`hhmmShort`/`formatStartTime`) in `DayCell`, `EventBlock`, `CreationPreview`, `DropPreview`, `CurrentTimeLine`, `TimeGridBackground`. **Check exact semantics per call site first** — the existing ones differ (e.g. `EventBlock` rounds the input, hours unpadded; `DayCell` unpads the hour of a `Date`). Unify to zero-padded `HH:MM` only if the visual diff is acceptable — otherwise keep a per-site variant.
   - Export `MS_PER_MINUTE = 60_000` — replaces `dropPayload.ts`'s private copy and the raw `60000`/`* 60000` in `DayColumn.svelte`.
3. **Frontend 3 — one constant/type home** (after F1):
   - Delete `DAYS_IN_WEEK` from `spanLayout.ts`; import from `date-grid/constants.ts`.
   - Import `MINUTES_PER_DAY` from `snapping.ts`; delete the copies in `dropPayload.ts` and `bucketing.ts`, and any in tests (`dropPayload.test.ts`, `bucketing.test.ts`, `spanLayout.test.ts` — check).
   - Delete `DayLayoutEvent` in `layout.ts`; `layoutEvents` accepts/returns `LaidEvent` (`types.ts`). Note `LaidEvent.lanes` is required but `layoutEvents` computes it — check the call site (`DayTimeline.svelte`) for how inputs arrive; may need `lanes` made optional on the input or a small adjustment.
   - Drop the `export { … } from './constants'` re-export block in `day-timeline/types.ts`; update importers to import from `./constants` (find them: grep `from '../types'` / `from './types'`).
4. **Frontend 12 — share the day-overlap rule:** in `bucketEventsByDay`, replace `if (start >= dayEnd || end <= dayStart) continue;` with `if (!instanceOccursOnDay(ev, date)) continue;` (import from `src/lib/domain/timing`), then keep only the `minutesIntoDay` math. Logic is currently identical for timed instances.
5. **Frontend 7 — `src/lib/queries.ts`** with `eventInstances(range, filters)`, `allEvents()`, `activeCalendars()`. **Read `DateGrid.svelte` and `DayTimeline.svelte` first** to pin the exact argument shapes (not read yet). `commands/tests.rs` keeps guarding the contract; don't rename commands.
6. **Deps — `tempfile`:** replace the three hand-rolled `scratch_dir` helpers in `store/tests.rs`, `settings/tests.rs`, `caldav/tests/mod.rs` with `tempfile::tempdir()` (already in the lockfile).

Then run: `bun run check`, `bun run lint`, `bun run test:unit` (frontend touched), plus `cargo clippy --all-targets -- -D warnings` + `cargo nextest run` after the tempfile item.

## Left to you (complex / design)

- **Backend 1** (consolidate IPC surface, dissolve `screens/plan/mod.rs`), **3** (split `store/ops.rs`), **4** (split the CLI), **7** (fixtures/mock-server consolidation), **8** (shared CalDAV session setup + password env var — behavior change), **9** (recurrence vocabulary across Rust + modal contract), **6** (`DEEP_LINK`: wire or delete — a decision).
- **Frontend 5** (unify drag controllers), **6** (recurring-move prompt), **8** (inspector state module), **9** (sidebar `persistState`), **10** (debounce primitive), **11** (filter/sort menu shell) — all behavior-adjacent refactors.
- **Frontend 4 — stub-route deletion: dropped at your request; do not do it.**
- **Dependencies:** `svelte-sonner`, `date-fns`, `bits-ui` adoptions (new deps / behavior changes) and the `ical-rs`/`libdav` spikes.
- **Docs 1** (one home per fact) — needs ownership judgments.

## Bookkeeping still owed

- Update root `AGENTS.md` for the five committed items:
  - line ~39: `events.ts` mirror bullet → `ipcEvents.ts` … `src-tauri/src/ipc_events.rs`
  - line ~58: `src-tauri/src/events.rs` bullet → `ipc_events.rs`
  - line ~66 (Event Name Constants) and ~80 (Cross-Window State): same rename
  - line ~46 area: `domain/` structure line — add `ids.rs`/`task.rs`/`event.rs` split
  - Typed Error Contract bullet: `sync` → `remote`
  - Deferred bullet: `rrule_utils` → `domain/timing/recurrence.rs`
  - (once F2 lands) `time.ts` helper list gains `parseLocalYmd`, `formatHhMm`, `MS_PER_MINUTE`; (once F1 lands) note `snapping.ts` in the plan-screen structure.
  - Check `src-tauri/AGENTS.md` + `src/AGENTS.md` for stale references (the `rrule_utils::truncate_before` mention there is now `domain::timing::truncate_before`).
- Mark items 2/5/10/11/12 done in `CLEANUP-BACKLOG.md` (and strike frontend 4 if you want the record to match).

## Quirks learned (save yourself the debugging)

- Repo `git diff` for `.ts` files renders **side-by-side plain text** — `git diff file | grep '^+'` finds nothing; use `git diff --stat` / `--no-pager diff | cat -A`.
- New files written via the `write` tool are LF; git warns they'll become CRLF on Windows. Cosmetic.
- Linker needs writable `TMP`: set `TMP`/`TEMP` to `src-tauri/target/cargo-tmp` before any linking cargo command (already done in the runs above).
- `cargo nextest run` regenerates bindings; `git status` after it is the drift check.
