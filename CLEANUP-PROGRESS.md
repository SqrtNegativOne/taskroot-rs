# Cleanup backlog — progress (handoff)

Status as of this note. Eleven backlog items are done and verified: five backend from
the first pass, F1 + F2 from the second, and F3, F12, F7 + `tempfile` this pass.
Nothing that was queued in the previous handoff remains. The complex/design items
listed under "Left to you" are still open.

## Done

| Item | Commit | Notes |
| --- | --- | --- |
| Backend 2 — delete `time_utils/` | `afc6c5c` | `rrule_utils.rs` → `domain/timing/recurrence.rs`. |
| Backend 11 — rename events modules | `9070290` | `ipc_events.rs` / `ipcEvents.ts`. |
| Backend 10 — split `domain/mod.rs` | `90ffed9` | `domain/{ids,task,event}.rs` + re-exports. |
| Backend 5 — split `ics/tests.rs` | `b96e631` | `tests/{mod,fixtures,read,write,lines,alarms}.rs`. |
| Backend 12 — `sync` → `remote` | `46b4aa2` | Wire-contract change, both sides moved. |
| Frontend 2 — `time.ts` helpers | `fc7bbd7` | `parseLocalYmd`, `formatHhMm`, `MS_PER_MINUTE` added (+ tests). Both `parseYmd` copies, every `new Date(\`${…}T00:00:00\`)` site, all six clock formatters and the `60000` copies replaced. **Visual decision (veto-able):** all formatters unified to zero-padded `HH:MM`, so `DayCell`/`CreationPreview`/`DropPreview` show `09:05` instead of `9:05`. |
| Frontend 1 — `snapping.ts` | `0d76261` | New `src/screens/plan/snapping.ts`: `MINUTES_PER_DAY`, `snapPixels`, `clampDuration`. Replaced `snappedDropMinute`, `EventBlock`/`DayColumn` snapping and the drag duration clamps. |
| Frontend 3 — constant/type home | this pass | `DAYS_IN_WEEK` only in `date-grid/constants.ts`; `MINUTES_PER_DAY` only in `snapping.ts`; `DayLayoutEvent` deleted in favour of `UnlaidEvent` (`Omit<LaidEvent, 'lane' \| 'lanes'>`); `types.ts` no longer re-exports constants. |
| Frontend 12 — day-overlap rule | this pass | `bucketing.ts` now calls `instanceOccursOnDay` for membership. Fixed `timing.ts` to use the wall-clock `d`..`d+1` boundary (was `+24 h`), so 23/25-hour DST days no longer leak events; two DST tests added and `MS_PER_DAY` dropped. |
| Frontend 7 — `lib/queries.ts` | this pass | `eventInstances(range, filters, query)`, `allEvents()`, `activeCalendars()`; all eight `get_active_calendars` sites, both plan views and the inspector's raw `query_events` migrated. |
| Deps — `tempfile` | this pass | Dev-dependency added; the three `scratch_dir` helpers in `store/tests.rs`, `settings/tests.rs`, `caldav/tests/mod.rs` replaced by `tempfile::tempdir()`. Kept each `TempDir` guard alive for the whole test. Clippy also caught double-borrows in the migrated account/discovery tests; those were fixed. The `scratch_dir` in `src-tauri/src/lib.rs` tests is out of scope. |

## Left to you (complex / design) — unchanged

- **Backend** 1 (IPC consolidation), 3 (`store/ops.rs` split), 4 (CLI split), 7 (fixtures/mock servers), 8 (CalDAV session + password env var), 9 (recurrence vocabulary), 6 (`DEEP_LINK` decision).
- **Frontend** 5 (drag controllers), 6 (recurring-move prompt), 8 (inspector state), 9 (sidebar `persistState`), 10 (debounce primitive), 11 (filter/sort menu shell). Frontend 4 (stub routes) is **dropped — do not do it**.
- **Dependencies:** `svelte-sonner`, `date-fns`, `bits-ui`, the `ical-rs`/`libdav` spikes. Note: the `timing.ts` day-boundary fix in F12 is a small part of what the `date-fns` item would also fix — don't double-count it.
- **Docs 1** (one home per fact).

## Bookkeeping

Done this pass: root `AGENTS.md` gained `queries.ts`, the `time.ts` helpers and
`snapping.ts`; `src/AGENTS.md` gained the helpers and the exact `d`..`d+1` DST
boundary; `CLEANUP-BACKLOG.md` marks F3/F7/F12 and `tempfile` done. `src-tauri/AGENTS.md`
was already correct (`domain::timing::truncate_before`).

## Verification (after each step)

```bash
bun run check
bun run lint
bun run test:unit          # frontend touched
cargo clippy --all-targets -- -D warnings   # src-tauri, after tempfile item
cargo nextest run          # src-tauri, after tempfile item (also regenerates bindings)
```

This pass: `bun run check` clean, `bun run test:unit` 89/89, `cargo clippy` clean,
`cargo nextest run` 250/250 with no binding or `docs/cli.md` drift.

## Quirks learned (save yourself the debugging)

- The agent shell may be bash or PowerShell. Set the temp variables in whichever is active: `export TEMP=… TMP=$TEMP` or `$env:TEMP=…; $env:TMP=$env:TEMP`. Setting the wrong shell's form fails silently and vitest dies with `EPERM mkdir %APPDATA%\Local\Temp`.
- `TEMP`/`TMP` must point inside the repo for `vitest` (used `src-tauri/target/vitest-tmp`) and for cargo linking **and for `tempfile::tempdir()`** (use `src-tauri/target/cargo-tmp` for both) — the agent sandbox is confined to the repo, so anything touching the system temp fails unless redirected.
- Repo `git diff` for `.ts` files renders side-by-side plain text — `git diff file | grep '^+'` finds nothing; use `--stat` / `--no-pager diff | cat -A`.
- New files written via the `write` tool are LF; git warns they'll become CRLF on Windows. Cosmetic.
- `cargo nextest run` regenerates bindings; `git status` after it is the drift check.
