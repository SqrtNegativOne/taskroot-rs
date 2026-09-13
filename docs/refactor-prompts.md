# Taskroot: persistence & settings refactor — remaining waves

Working document for the **remaining** work only. Completed waves (P1, P2, P3,
P4, P5) have been removed along with their prompts; recover them from git history
if a future change needs the original wording.

## Status

| Wave | Prompt | State | Depends on |
|---|---|---|---|
| 5 | **P9 Independent verification** | ⬜ last | all |

Completed:

- **P1 — Settings substrate** (serde merge + `ui_state` table).
- **P2 — Setting metadata single source** (`#[setting(..)]` + `SettingsMeta`). The
  serialized schema contract (camelCase + `type` keys) is pinned by
  `src-tauri/src/settings/metadata/tests.rs`; do not break it.
- **P3 — Shared backend-hydration primitive** (`src/lib/asyncState.svelte.ts`:
  `createStaleGuard`, `createDebouncedWriter`, `hydrateOnce`; `persistState` and
  `useTauriQuery` now compose it).
- **P4 — Sidebar off `localStorage`** (`src/screens/sidebar/state.svelte.ts`):
  `ui.sidebar.*` is the source of truth, seeded once from the legacy `sidebar_*`
  keys only when the backend has no row, then the legacy keys are removed.
- **P5 — Command-layer test harness** (`src-tauri/src/test_support/`,
  `commands/tests.rs`, `settings/tests.rs`): handlers are documented as thin
  wrappers over `&SqlitePool` bodies, and the JS↔Rust command contract is pinned
  by source-scanning registry tests. No mock-runtime helper: all commands take a
  concrete `AppHandle<Wry>`, which `MockRuntime` cannot satisfy without changing
  38 public signatures.

Remaining waves run **serially** in one working directory, in the order above;
P9 must see the merged result of everything before it. Do not start a wave until
the previous one is green and committed.

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

---

## PROMPT P9 — Independent verification (read-only)

```text
Role
Independent verifier. You wrote none of these changes. Be adversarial. Do not fix
anything; produce a findings report.

Scope
Review the merged result of P1–P5 against their acceptance criteria and the repo rules in
AGENTS.md / src/AGENTS.md / src-tauri/AGENTS.md.

Check specifically
1. Settings substrate (P1): does `get_settings` still silently drop values it cannot merge?
   Prove it with a test or counterexample. Is the settings-vs-ui_state rule documented and
   followed? Any second JSON-decode rule left in `settings.rs`?
2. Metadata single source (P2): is there a test that fails when a new `AppSettings` field is
   added but not described? Do ids/labels/option values/defaults match the pre-refactor output?
3. Hydration primitive (P3): are the three original copies unified, or is `persistState`
   still carrying its own state machine? Is `store.svelte.ts` still idempotent?
4. Channels (P4): any remaining `localStorage` write for user-facing state? Does the
   one-time migration seed correctly and then stop consulting localStorage?
5. Command tests (P5): if a smoke test was skipped in favour of a registry test, is the
   documented reason honest, or is it hiding an untested command surface? A green suite must
   not be presented as more coverage than it is.

Run the full gate yourself and report the raw result
  bun run check && bun run lint && bun run test:unit
  cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo nextest run
  git diff --exit-code src/lib/bindings
  bun run check:deps

Output
- Ordered findings: blocking / should-fix / nit.
- For each: file:line, why it violates the stated criterion, and the smallest repro or test.
- Explicitly list which acceptance criteria you could NOT verify, and why.
- Propose the minimal fix per finding; no broad rewrites.
```
