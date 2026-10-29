# Cleanup backlog — progress (handoff)

Status as of this note. Seven backlog items are committed and verified (five
backend from the previous pass, F1 + F2 this pass). Remaining planned items:
F3, F12, F7, tempfile, then the full verification suite and bookkeeping.

## Done (committed on `main`)

| Item | Commit | Notes |
| --- | --- | --- |
| Backend 2 — delete `time_utils/` | `afc6c5c` | `rrule_utils.rs` → `domain/timing/recurrence.rs`. |
| Backend 11 — rename events modules | `9070290` | `ipc_events.rs` / `ipcEvents.ts`. |
| Backend 10 — split `domain/mod.rs` | `90ffed9` | `domain/{ids,task,event}.rs` + re-exports. |
| Backend 5 — split `ics/tests.rs` | `b96e631` | `tests/{mod,fixtures,read,write,lines,alarms}.rs`. |
| Backend 12 — `sync` → `remote` | `46b4aa2` | Wire-contract change, both sides moved. |
| Frontend 2 — `time.ts` helpers | `fc7bbd7` | `parseLocalYmd`, `formatHhMm`, `MS_PER_MINUTE` added to `src/lib/time.ts` (+ tests in `time.test.ts`). Replaced both `parseYmd` copies, all ``new Date(`${…}T00:00:00`)`` sites, all six clock formatters and the `60000`/`MS_PER_MINUTE` copies. **Visual decision made (flag to user, veto-able):** all six formatters unified to zero-padded `HH:MM`, so `DayCell`, `CreationPreview`, `DropPreview` now show `09:05` instead of `9:05` (matches the padded timeline hour labels). `domain/events.ts` keeps its own `MS_PER_MINUTE` (out of scope per plan). |
| Frontend 1 — `snapping.ts` | `0d76261` | New `src/screens/plan/snapping.ts`: `MINUTES_PER_DAY`, `snapPixels(px)`, `clampDuration(mins)` (imports `PX_PER_MIN`/`SNAP_MIN` from `day-timeline/constants.ts`, which keeps the pixel constants). Replaced: `snappedDropMinute` core (keeps its [0, day−fitted] clamp), `EventBlock` `snappedDeltaMinutes`, `DayColumn` `snappedMinutesFromOffset`, duration clamps in `eventDrag`/`taskDrag` and `timelineDropPayload`. `dropPayload.ts` no longer exports `MINUTES_PER_DAY`. `bun run check` + plan/time unit tests green; full lint/nextest still owed (see verification). |

## Not started — planned, ready to execute

1. **Frontend 3 — one constant/type home.**
   - `spanLayout.ts`: delete `export const DAYS_IN_WEEK = 7;`, import from `./constants` (date-grid). `DateGrid.svelte` already imports it from there.
   - `bucketing.ts`: delete `const MINUTES_PER_DAY = HOURS_PER_DAY * MINUTES_IN_HOUR;`, import `MINUTES_PER_DAY` from `../snapping` (keep the `HOURS_PER_DAY`/`MINUTES_IN_HOUR` imports only if still used — `minutesIntoDay` uses neither after this). `bucketing.test.ts` has its own local `MINUTES_IN_HOUR`/`HOURS_PER_DAY`/`MINUTES_PER_DAY` (lines 6–8) — point the test at `snapping.ts` too.
   - `layout.ts`: delete `DayLayoutEvent`. Add `export type UnlaidEvent = Omit<LaidEvent, 'lane' | 'lanes'>;` to `day-timeline/types.ts` (next to `LaidEvent`); `layoutEvents(events: UnlaidEvent[]): LaidEvent[]`; `bucketing.ts` returns `Record<string, UnlaidEvent[]>`. Only call site is `DayTimeline.svelte::planLayout` (already typed `Record<string, LaidEvent[]>`).
   - `day-timeline/types.ts`: delete the `export { … } from './constants'` re-export block. Its only constant consumer is `hooks/useCurrentTime.svelte.ts` (`MINUTES_IN_HOUR`) — switch that import to `../constants`. No other file pulls constants via `types`.
2. **Frontend 12 — share the day-overlap rule.** In `bucketing.ts` replace `if (start >= dayEnd || end <= dayStart) continue;` with `if (!instanceOccursOnDay(ev, date)) continue;` (import from `../../../lib/domain`), keep the `minutesIntoDay` math. **Gotcha found:** the two boundaries are NOT identical on DST days. `instanceOccursOnDay` (`src/lib/domain/timing.ts`) builds `dayEnd = dayStart + 24h` — the exact defect flagged in the backlog's `date-fns` note — while `bucketing` uses the wall-clock `new Date(y, m, d+1)`. Fix `timing.ts` to the `d+1` boundary **first** (drop the now-unused `MS_PER_DAY`), then swap; otherwise the swap regresses near-midnight events on 23/25-hour days. `timing.test.ts` has no DST dates today — add one fall-back (e.g. 2026-11-01 US) edge case. Note the all-day branch is unreachable from bucketing (timed-only loop).
3. **Frontend 7 — `src/lib/queries.ts`.** Arg shapes now pinned (read `DateGrid.svelte` + `DayTimeline.svelte`):
   - `query_event_instances`: `{ filters: AppEventFilter[], query: string, startDate: string, endDate: string }` (both plan views, `debounceMs: 150`).
   - `get_active_calendars`: `{}` — seven call sites: `PlanScreen`, `DateGrid`, `DayTimeline`, `Sidebar`, `TaskListPane`, `InspectorPane`, `InspectorEventFields`, `InspectorTaskFields` (grep, 8 rows total).
   - `query_events` (raw, inspector): `{ filters: [], query: "" }` in `InspectorPane.svelte` → `allEvents()` builder.
   - Builders: `eventInstances(range, filters, query)`, `allEvents()`, `activeCalendars()`. `AppEventFilter` type re-exported by the `src/lib/domain` barrel. Do not rename commands; `commands/tests.rs` guards the contract.
4. **Deps — `tempfile`:** add to `[dev-dependencies]` in `src-tauri/Cargo.toml` (3.27.0 already in the lockfile). Replace the three `scratch_dir` helpers: `store/tests.rs:92`, `settings/tests.rs:8`, `caldav/tests/mod.rs:34` (used by `caldav/tests/{account,discovery}.rs`). **Lifetime gotcha:** the `TempDir` guard must outlive every disk access — `settings/tests.rs::state(name)` returns a `StoreState` whose `store.data_dir` is written later, so restructure each test to create the tempdir first and hold the guard; the `name` args exist only to make dirs unique, so drop them. `store/tests.rs::store_with_series` may drop the guard (only `Store::load` reads). The `caldav/tests/mod.rs` sandbox doc-comment on `scratch_dir` goes stale (tempfile honours `TEMP`/`TMP`). A 4th `scratch_dir` exists in `src/lib.rs:341` — out of scope per the backlog, noted only.

Then run the full suite (below). No TS `#[derive(TS)]` structs are touched by F3/F7, so `cargo nextest run` binding drift is not expected from the frontend items — only the tempfile commit needs `clippy` + `nextest`.

## Left to you (complex / design) — unchanged

- **Backend** 1 (IPC consolidation), 3 (`store/ops.rs` split), 4 (CLI split), 7 (fixtures/mock servers), 8 (CalDAV session + password env var), 9 (recurrence vocabulary), 6 (`DEEP_LINK` decision).
- **Frontend** 5 (drag controllers), 6 (recurring-move prompt), 8 (inspector state), 9 (sidebar `persistState`), 10 (debounce primitive), 11 (filter/sort menu shell). Frontend 4 (stub routes) is **dropped — do not do it**.
- **Dependencies:** `svelte-sonner`, `date-fns`, `bits-ui`, the `ical-rs`/`libdav` spikes. Note: the `timing.ts` day-boundary fix in F12 is a small part of what the `date-fns` item would also fix — don't double-count it.
- **Docs 1** (one home per fact).

## Bookkeeping still owed

- Root `AGENTS.md`: bullets for `ipcEvents.ts`/`ipc_events.rs`, `domain/{ids,event,task}.rs` split, `sync` → `remote`, `rrule_utils` → `domain/timing/recurrence.rs` (Deferred bullet); add `parseLocalYmd`/`formatHhMm`/`MS_PER_MINUTE` to the `time.ts` bullet and `snapping.ts` to the plan-screen structure line (both landed this pass). Check `src/AGENTS.md` + `src-tauri/AGENTS.md` for stale refs (`rrule_utils::truncate_before` → `domain::timing::truncate_before`).
- `CLEANUP-BACKLOG.md`: mark Frontend 1 (`0d76261`) and Frontend 2 (`fc7bbd7`) done; mark 3/12/7 as they land.

## Verification (after each step)

```bash
bun run check
bun run lint
bun run test:unit          # frontend touched
cargo clippy --all-targets -- -D warnings   # src-tauri, after tempfile item
cargo nextest run          # src-tauri, after tempfile item (also regenerates bindings)
```

## Quirks learned (save yourself the debugging)

- The sandbox shell here is **bash**, not PowerShell: `export TEMP=… TMP=$TEMP` — `$env:TEMP=…` fails silently and vitest dies with `EPERM mkdir %APPDATA%\Local\Temp`.
- `TEMP`/`TMP` must point inside the repo for `vitest` (used `src-tauri/target/vitest-tmp`) and for cargo linking **and for `tempfile::tempdir()`** (use `src-tauri/target/cargo-tmp` for both) — the agent sandbox is confined to the repo, so anything touching the system temp fails unless redirected.
- Repo `git diff` for `.ts` files renders side-by-side plain text — `git diff file | grep '^+'` finds nothing; use `--stat` / `--no-pager diff | cat -A`.
- New files written via the `write` tool are LF; git warns they'll become CRLF on Windows. Cosmetic.
- `cargo nextest run` regenerates bindings; `git status` after it is the drift check.
