# Taskroot: persistence & settings refactor — P9 follow-ups

P1–P5 are complete, committed, and independently verified (P9, read-only). No
planned waves remain. This file now tracks P9's residual findings only; recover
the original P1–P5 prompts from git history if needed.

## P9 result

Full gate green on the P1–P5 result, with no blocking findings:

- `bun run check` → 0 errors / 0 warnings
- `bun run lint` → clean
- `bun run test:unit` → 53 passed
- `cargo clippy --all-targets -- -D warnings` → clean
- `cargo nextest run` → 129 passed
- `git diff --exit-code src/lib/bindings` → clean
- `bun run check:deps` → clean

Verified: the settings merge drops only unmergeable legacy rows (documented
contract), the metadata schema strings match the pre-P2 output, `persistState`
no longer owns a timer/flag state machine and `useTauriQuery` uses the stale
guard, `store.svelte.ts` is still idempotent, the sidebar has no user-facing
`localStorage` write left, and the one-time migration seeds only when the backend
is empty. The P5 "no mock-runtime" reason is honest (all commands take a concrete
`AppHandle<Wry>`).

## Follow-ups

### Should-fix

- **F1 — frontend arg-contract scan is blind to `.execute({...})` arguments.**
  `src-tauri/src/test_support/frontend_scan.rs` only reads argument literals at
  the wrapper call site, so the three `useTauriQuery(...).execute({...})` sites
  (`src/routes/minitracker/+page.svelte`, `src/screens/do/DoScreen.svelte`,
  `src/screens/do/stopwatch/Stopwatch.svelte`) are recorded with no keys and are
  never assertion-checked. Fix: pass args through `options.args`, or bind
  `.execute({...})` to the preceding command literal in the scanner; at minimum
  document the gap in `src-tauri/AGENTS.md`.
- **F2 — no `commands::events` command body is exercised.**
  `commands/tests.rs`'s `get_active_calendars_returns_every_stored_calendar` calls
  `db::get_calendars`, not the handler body, so no events command has a covered
  `&SqlitePool` body. Fix: extract `active_calendars(&SqlitePool)` in
  `commands/events.rs` and call it from both the handler and the test; or rename
  the test and state that no events handler has an extracted body.

### Nits

- **N1 — P3 unification is partial.** `store.svelte.ts` still owns its idempotent
  `initPromise` bootstrap rather than `hydrateOnce`; the P3 out-of-scope note
  cites a `'not-ready'` retry loop that no longer exists.
- **N2 — P2 output is not pinned byte-for-byte.** `settings/metadata/tests.rs`
  pins ids, `defaultValue` and option presence, but not label/option/keyword
  strings; a snapshot would close the gap.
- **N3 — `update_setting` still accepts unknown keys**, writing rows
  `get_settings` will never merge (`settings/storage.rs`); reject them or document
  the intentional loose setter.
- **N4 — source-scan heuristics are brittle.** `source_scan.rs`'s
  `between(..., ']')`, comma-split params, and missing-required-argument blindness
  should be listed as limits in `src-tauri/AGENTS.md`.
- **N5 — `src-tauri/AGENTS.md` overstates the mock-runtime cost** ("38 commands";
  only the ~33 that take `app` would need to be runtime-generic).

## Rules (every prompt)

- Read `AGENTS.md`, `src/AGENTS.md`, `src-tauri/AGENTS.md` before editing.
- Never hand-edit `src/lib/bindings/*.generated.ts`; regenerate with
  `cargo nextest run` in `src-tauri`, then commit.
- Rust denies `unwrap_used`, `expect_used`, `indexing_slicing`,
  `arithmetic_side_effects`, `as_conversions`, `todo`, `panic`, `exit`. Test
  modules may opt out with `#![allow(clippy::unwrap_used, clippy::expect_used)]`.
- Frontend: Svelte 5 runes only; `neverthrow` `Result` for expected errors;
  `safeInvoke`/`useTauriQuery` for IPC; no `any`.
- Every IPC command returns `Result<T, AppError>` (`{code, message}`); never a raw
  string.
- Files > 250 LOC: split. Prefer early returns. No tautological tests.
- Update the relevant `AGENTS.md` for any new file, table, command, or convention.

## Definition of done (every prompt)

```sh
bun run check
bun run lint
bun run test:unit
cd src-tauri && cargo clippy --all-targets -- -D warnings
cd src-tauri && cargo nextest run        # also regenerates bindings
git diff --exit-code src/lib/bindings    # binding-drift gate
```

Stop and report if: a module the prompt assumes does not exist; the fix requires
renaming a public command or changing the settings-screen contract; a test must be
deleted or weakened; or there is user-visible data-loss risk.
