# Taskroot: persistence & settings refactor — remaining waves

Working document for the **remaining** work only. Completed waves (P1, P2, P3)
have been removed along with their prompts; recover them from git history if a
future change needs the original wording.

## Status

| Wave | Prompt | State | Depends on |
|---|---|---|---|
| 3 | **P5 Command-layer test harness** | ⬜ next | — |
| 4 | **P4 Retire sidebar `localStorage`** | ⬜ next | P1 + P3 |
| 5 | **P9 Independent verification** | ⬜ last | all |

Completed:

- **P1 — Settings substrate** (serde merge + `ui_state` table).
- **P2 — Setting metadata single source** (`#[setting(..)]` + `SettingsMeta`). The
  serialized schema contract (camelCase + `type` keys) is pinned by
  `src-tauri/src/settings/metadata/tests.rs`; do not break it.
- **P3 — Shared backend-hydration primitive** (`src/lib/asyncState.svelte.ts`:
  `createStaleGuard`, `createDebouncedWriter`, `hydrateOnce`; `persistState` and
  `useTauriQuery` now compose it).

Remaining waves run **serially** in one working directory, in the order above:
P5 and P4 touch disjoint files but P5 may add command-level test support that P4's
verification can reuse, and P9 must see the merged result of both. Do not start a
wave until the previous one is green and committed.

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

## PROMPT P5 — Command-layer test harness + smoke tests

```text
Goal
37 `#[tauri::command]` functions exist and none are tested against an `AppHandle` or a
mock runtime. A typo in `generate_handler!` or a JS/Rust argument-casing mismatch is
invisible to `cargo nextest` until runtime. Give the command layer a paved road.

Read first
- src-tauri/src/lib.rs (`generate_handler!`, `db_pool`)
- src-tauri/src/commands/*.rs, src-tauri/src/settings/, src-tauri/src/screens/plan/mod.rs
- src-tauri/src/db/mod.rs (tests use `init_db("sqlite::memory:")`)
- src-tauri/Cargo.toml (note: `tauri` has no `test` feature enabled yet)

Required outcome
- A documented testing convention in src-tauri/AGENTS.md: commands are thin wrappers over
  `async fn(&SqlitePool, ...)` functions living in the testable layer; the command body
  does arg marshalling + `db_pool(&app)` only. Extract bodies where that is not yet true
  (behaviour-preserving).
- A reusable test helper that builds an in-memory database pool and a mock Tauri app, so
  a command can be invoked end-to-end. Use `tauri::test::mock_builder` / `mock_context`;
  enable the `tauri` `test` feature for tests only ([dev-dependencies] entry or a
  `test-support` feature — never release builds).
- If a full mock-webview round trip is impractical on this platform, fall back to:
  (a) extracting the command body into an `&SqlitePool` function and testing that, and
  (b) a minimal registry test asserting every command name in `generate_handler!` is
  unique and snake_case, plus a test that JS-facing argument names match the Rust
  parameters. Say clearly in AGENTS.md which approach is used and why.

Smoke tests (at minimum)
- `settings::get_setting` / `settings::update_setting` round trip through the real handler.
- One `commands::tasks` command and one `commands::events` command round trip.
- `get_settings` returns defaults when the table is empty.

Constraints
- Do not change production behaviour to make tests easier, except the thin-wrapper
  extraction.
- Keep test modules `#![allow(clippy::unwrap_used, clippy::expect_used)]` and otherwise
  clean under `cargo clippy --all-targets -- -D warnings`.
- If you refactor `settings.rs` or `screens/plan/mod.rs`, coordinate so P3 (frontend-only)
  is not running concurrently.

Verify
  cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo nextest run
  git diff --exit-code src/lib/bindings
Update src-tauri/AGENTS.md with the command-testing convention.
```

---

## PROMPT P4 — Consolidate persistence channels (retire sidebar `localStorage`)

```text
Goal
There are two persistence channels with no written rule: backend SQLite for app settings
and per-component UI state, and `localStorage` for sidebar state. Pick one rule, document
it, and migrate the outlier.

Current localStorage usage (src/screens/sidebar/Sidebar.svelte)
- `sidebar_tab_top`    (tab position, float)
- `sidebar_notes`      (notes text)
- `sidebar_show_notes` (notes pane open/closed)
(src/routes/dev/+page.svelte is the dev inspector — leave it.)

Required outcome
- A written rule in src/AGENTS.md: backend SQLite is the source of truth for anything that
  must survive a webview data reset or be shared across windows; `localStorage` is not used
  for user-facing state.
- Migrate the three values to the backend settings/UI-state API (P1), using the P3
  primitive. Keep the draggable tab behaviour identical.
- On first run after migration, seed from any existing `localStorage` values so current
  users do not lose their sidebar position/notes; then stop writing to `localStorage`.
- `notesText` is free-form and written on every keystroke today: use the debounced
  write-back from the primitive; do not write per keystroke.

Constraints
- The sidebar is a separate Tauri webview (`label === 'sidebar'`); the value must be
  readable from that window via the shared backend, not window-scoped storage.
- Do not change the sidebar's window-sizing / monitor logic.
- Depends on P1 (storage API) and P3 (primitive). Do not start before both are merged.

Tests
- Migration seeds from pre-existing localStorage values when the backend key is absent.
- After migration the backend value wins and localStorage is not consulted.
- Round-trip of the notes text and the tab position through the backend.

Verify
  bun run check && bun run lint && bun run test:unit
Update src/AGENTS.md with the channel rule.
```

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
