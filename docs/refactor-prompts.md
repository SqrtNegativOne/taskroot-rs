# Taskroot: persistence & settings refactor — agent prompts

Working document. Derived from a review of the `settings` / persistence subsystem after
implementing persisted filters, sorts, view modes, pane sizes and collapse state.

Copy exactly one prompt per agent run. Each prompt is self-contained.

---

## 0. Prerequisite: commit the in-flight change first

The current working tree contains the change that introduced per-component UI persistence.
Commit it before starting so every agent gets a clean base and diffs stay reviewable.

What that change added (referred to by the prompts):

- `src-tauri/src/settings.rs` — new `get_setting(key) -> Value` command and `parse_setting_value`.
- `src-tauri/src/lib.rs` — `settings::get_setting` registered in `generate_handler!`.
- `src/lib/persisted.svelte.ts` — `persistState` rune + `persistedKeys` (keys namespaced `ui.*`).
- `vite.config.js` — `resolve.conditions: ['browser']` under `process.env.VITEST`.
- Tests — `src/lib/persisted.test.ts`, `src-tauri/src/settings.rs` unit tests, settings roundtrip in `src-tauri/src/db/mod.rs`.
- `AGENTS.md` / `src/AGENTS.md` — documented the persisted-UI-state rule.

---

## 1. Rules every prompt must follow

- Read `AGENTS.md`, `src/AGENTS.md`, `src-tauri/AGENTS.md` before editing.
- Never hand-edit `src/lib/bindings/*.generated.ts`. Run `cargo nextest run` in `src-tauri` to regenerate, then commit.
- Strict backend lints: `unwrap_used`, `expect_used`, `indexing_slicing`, `arithmetic_side_effects`, `as_conversions`, `todo`, `panic`, `exit` are **denied**. Test modules may opt out with `#![allow(clippy::unwrap_used, clippy::expect_used)]`.
- Frontend: Svelte 5 runes only; `neverthrow` `Result` for expected errors; `safeInvoke`/`useTauriQuery` for IPC; no `any`.
- Every IPC command returns `Result<T, AppError>` (`{code, message}`); never a raw string.
- Files > 250 LOC should be split; prefer early returns; do not add tautological tests.
- Update the relevant `AGENTS.md` when architecture or file structure changes.
- **One writer per working directory.** Prompts marked `PARALLEL` must run in isolated worktrees.

Definition of done for **every** prompt:

```sh
bun run check
bun run lint
bun run test:unit
cd src-tauri && cargo clippy --all-targets -- -D warnings
cd src-tauri && cargo nextest run        # also regenerates bindings
git diff --exit-code src/lib/bindings    # binding-drift gate
```

---

## 2. Execution order

| Wave | Prompt | Agent | Isolation | Depends on | Why here |
|---|---|---|---|---|---|
| 1 | **P1** Settings substrate: serde-driven merge + first-class UI state | worker | solo (single writer) | — | Foundation; rewrites `settings.rs` read/write path |
| 1-gate | Review P1 | oracle (read-only) | — | P1 | Highest-stakes change; verify before building on it |
| 2 | **P2** Single source of truth for setting metadata | worker (ideally same agent resumed) | solo | P1 merged | Rewrites `get_settings_schema`; same file as P1 → must not run concurrently |
| 2-gate | Review P2 | reviewer | — | P2 | Schema is user-facing and drift-prone |
| 3 | **P3** Shared backend-hydration primitive + `persistState` refactor | worker | PARALLEL (worktree) | P1 merged | Frontend-only; build on final `get_setting` shape |
| 3 | **P5** Command-layer test harness + smoke tests | worker | PARALLEL (worktree) | P1 + P2 merged | Tests the settled command surface; different files than P3 |
| 4 | **P4** Consolidate persistence channels (retire sidebar `localStorage`) | worker | solo | P1 + P3 merged | Consumes the P3 primitive and the P1 storage API |
| 5 | **P9** Independent verification of P1–P5 | oracle (read-only) | — | all merged | Final adversarial pass before declaring done |

Notes:

- P1 → P2 must be **sequential** (same file). If the same worker agent can be resumed for P2, do that so it keeps `settings.rs` in context.
- P3 and P5 only overlap if P5 chooses to refactor command bodies. If it does, add `src-tauri/src/settings.rs` and `src-tauri/src/screens/plan/mod.rs` to P5's diff — then run P3 and P5 sequentially too.

---

## PROMPT P1 — Settings substrate: serde-driven merge + first-class arbitrary UI state

```text
Goal
Fix the settings persistence substrate so it (a) merges stored values correctly for all
serde-representable types and (b) supports first-class storage of arbitrary per-component
UI state, instead of the current "ui.* smuggling" into a table whose main reader ignores it.

Read first
- src-tauri/src/settings.rs        (AppSettings struct, get_settings, get_settings_schema,
                                     update_setting, get_setting, parse_setting_value)
- src-tauri/src/db/settings.rs     (get_setting / set_setting / delete_setting)
- src-tauri/src/db/mod.rs          (init_db: `settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)`)
- src/lib/store.svelte.ts          (store.refresh -> get_settings)
- src/lib/persisted.svelte.ts      (persistState -> get_setting/update_setting, keys `ui.*`)
- src/routes/settings/+page.svelte (calls get_settings / update_setting / get_settings_schema)
- src-tauri/AGENTS.md, src/AGENTS.md

Problems to solve
1. `get_settings` re-implements type knowledge by hand:
     matches!((default, stored), (Bool,Bool) | (Number,Number) | (String,String))
   Anything else (arrays, objects, null) is silently dropped. A stored value whose type
   legitimately changed (e.g. number -> string) is also silently dropped.
2. `load_stored_settings` and `parse_setting_value` both parse stored JSON with
   `serde_json::from_str(..).unwrap_or(Value::String(..))`. Two copies of one rule.
3. Arbitrary UI state currently lives under `ui.*` keys in the same table, readable only
   through the separate `get_setting` command. `get_settings` ignores those rows.
   There is no documented rule for what belongs where.
4. `settings.rs` is the only module that bypasses `crate::db_pool(&app)`; it re-implements
   `app.try_state::<SqlitePool>().ok_or_else(..)` twice. Every other command module uses
   the helper. Fix that while you are here.

Required outcome
- One JSON decode helper used by both read paths. No duplicated `unwrap_or(Value::String)`.
- `get_settings` merges stored values through serde, not a hand-written type match.
  Design it so the merge rule is defined once and stays correct when the struct changes.
  At minimum: a stored value that fails to deserialize into the field must not be merged,
  and a value that deserializes must be. Prefer deserializing the merged JSON into
  `AppSettings` and rejecting only the keys that fail, or validate per-field via a
  serde round-trip. Do not silently drop arrays/objects/booleans you did not enumerate.
- Arbitrary UI state gets a first-class, documented home. Pick ONE and justify it in a
  short comment + AGENTS.md note:
    Option A: a dedicated `ui_state (key TEXT PRIMARY KEY, value TEXT NOT NULL)` table with
              its own `get_ui_state` / `set_ui_state` commands.
    Option B: keep the `settings` table but give `AppSettings` an explicit nested
              `ui: BTreeMap<String, Value>` (or `#[serde(flatten)]`) so `get_settings`
              returns it and `get_setting` is no longer a second contract on one table.
  Whichever you choose, `persistState` must keep working (update it if the command names
  change) and the "what goes where" rule must be written down.
- `settings.rs` uses `crate::db_pool(&app)?` like every other module.

Constraints
- Keep the existing command names `get_settings` / `update_setting` stable unless the chosen
  option genuinely requires a rename; if you rename, update every caller.
- `AppSettings` keeps `#[derive(TS)]`; if its shape changes, run `cargo nextest run` and
  commit the regenerated `src/lib/bindings/AppSettings.generated.ts`.
- No behaviour change to the Settings screen other than the substrate.
- Do not solve this by adding more `ui.*` special cases.

Tests (write/tighten; behaviour, not implementation)
- A stored array/object round-trips through the chosen UI-state path (store -> read back).
- A stored value of the wrong type for a known setting is ignored, and the default is used.
- A stored value of the correct type overrides the default.
- Deleting the key restores the default.
- The JSON decode helper: valid JSON decodes; invalid JSON falls back as before; missing
  key yields Null for the single-key read and the default for the aggregate read.

Out of scope
- Deriving `get_settings_schema` from the struct (that is P2, and it touches the same file —
  do not start it here).
- Migrating sidebar localStorage (that is P4).

Verify
  bun run check && bun run lint && bun run test:unit
  cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo nextest run
  git diff --exit-code src/lib/bindings
Update AGENTS.md for any new table/command/file-structure change.
```

---

## PROMPT P2 — Single source of truth for setting metadata (schema)

```text
Goal
Stop maintaining every setting in three places. Today a setting needs:
  1. a field + `Default` value in the `AppSettings` struct (src-tauri/src/settings.rs),
  2. a hand-written entry in the giant `json!` in `get_settings_schema` (label, keywords,
     type, min/max, options, and a `defaultValue` re-derived from `defaults.<field>`),
  3. (when typed) a regenerated TS binding.
Nothing keeps them in sync. `get_settings_schema` carries `#[allow(clippy::too_many_lines)]`
and the file opens with `#![allow(clippy::struct_excessive_bools)]`.

Key discovery — the repo already has the pattern you should copy
`src-tauri/taskroot-macros/src/lib.rs` defines a `Queryable` derive that, from a struct with
`#[query(...)]` field attributes, generates:
  - `{Name}FilterColumn` enum
  - `{Name}ColumnDef` + `{Name}::get_schema()`
  - `{Name}Filter` / `{Name}Sort`
It is already used on `AppTask` and `AppEvent` (src-tauri/src/domain/mod.rs) and exposed via
`get_task_schema()` / `get_event_schema()` (src-tauri/src/screens/plan/mod.rs). Settings
should follow the same struct -> metadata approach instead of a hand-written `json!`.

Additional cleanup in scope
- `src/routes/settings/schema.ts` documents itself as:
    "Generated from src-tauri/settings.yaml by src-tauri/build.rs (ts-rs export)."
  No such YAML exists and `src-tauri/build.rs` is just `tauri_builder::build()`. The comment
  is a leftover from a removed codegen pipeline (commit 1b6e474). Fix or delete it.

Read first
- src-tauri/src/settings.rs (AppSettings + get_settings_schema)
- src-tauri/taskroot-macros/src/lib.rs, src-tauri/src/domain/mod.rs (Queryable usage)
- src-tauri/src/screens/plan/mod.rs (get_task_schema)
- src/routes/settings/+page.svelte, SettingRow.svelte, schema.ts
- src/lib/bindings/AppSettings.generated.ts

Required outcome
- Setting metadata (label, keywords, type, options, min/max, danger, section, tab) is declared
  next to the field, not duplicated in a `json!` blob. Reuse the `Queryable` derive style —
  either extend `taskroot-macros` with a settings-oriented derive or add a focused
  `SettingMeta` derive. Keep it small and typed.
- `get_settings_schema` is generated from that metadata. Its output JSON must remain
  byte-compatible with the current contract consumed by `src/routes/settings/schema.ts`
  (`tabs[].sections[].settings[]` with `id,label,description,keywords,type,options,min,max,
  defaultValue,danger`) unless you also update the consumer.
- `defaultValue` comes from `AppSettings::default()` — not re-declared.
- The "custom" pseudo-settings (`logout`, `clear_all_data`) remain representable.
- Remove the now-unnecessary `#[allow(clippy::too_many_lines)]` if the code shrinks enough.

Constraints
- Do not change setting ids, labels, option values, or defaults. The Settings UI must look
  and behave identically.
- If you touch `taskroot-macros`, do not break `AppTask`/`AppEvent` codegen.
- `cargo nextest run` regenerates bindings; commit any drift.
- If P1 is not yet merged, stop — this prompt rewrites the same file.

Tests
- A test asserts every `AppSettings` field appears exactly once in the generated schema
  (this is the anti-drift test that makes the whole change worth it).
- A test asserts schema `defaultValue` equals the corresponding `AppSettings::default()` field.
- Existing settings-schema tests (if any) stay green.

Out of scope
- Changing the storage/merge behaviour (P1).
- Adding new settings.

Verify
  bun run check && bun run lint && bun run test:unit
  cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo nextest run
  git diff --exit-code src/lib/bindings
Update AGENTS.md: document how a new setting is now added.
```

---

## PROMPT P3 — Shared backend-hydration primitive; refactor `persistState`

```text
Goal
Extract the repeated "read once from the backend, then keep a reactive value in sync"
logic into one documented primitive, and refactor the existing copies onto it.

Current duplication (all three encode the same idea)
- src/lib/store.svelte.ts   — bespoke idempotent `init()` with a cached bootstrap promise
                              (no retry loop; the `'not-ready'` code no longer exists).
- src/lib/safeInvoke.svelte.ts — `useTauriQuery` with a bespoke stale-guard request-id
                              counter + `store-updated` listener.
- src/lib/persisted.svelte.ts — `persistState` with a bespoke hydration flag, debounce
                              timer, unchanged-value suppression and `onDestroy` flush.

Required outcome
- A small, documented module (e.g. `src/lib/asyncState.svelte.ts`) exporting the shared
  pieces: hydrate-once-from-command, stale-response guarding, debounced write-back,
  flush-on-destroy. Keep it minimal — do not build a framework.
- `persistState` is re-implemented on top of it, with identical external behaviour
  (hydrate once, suppress writes while hydrating, skip unchanged values, flush on destroy,
  optional validator). All existing `persistedKeys` call sites keep working unchanged.
- Decide and document where `persistState` belongs (it currently sits directly in
  `src/lib/`). Keep `src/lib/persisted.svelte.ts` as the public entry point unless you have
  a strong reason, and say so.
- Consider exposing a bindable shape (return a `$state` box) in addition to the current
  `(read, write)` callbacks if it simplifies call sites — but do not churn all components
  for style. Only change call sites where it removes real duplication.

Constraints
- No behaviour regressions: the 5 tests in `src/lib/persisted.test.ts` must pass unchanged
  (they encode the contract).
- Do not touch `settings.rs` (that is P1/P2). This is frontend-only.
- Do not migrate sidebar localStorage (that is P4).

Tests (behaviour, not implementation)
- Keep all existing `persistState` tests green.
- Add tests for the extracted primitive's stale-response guard and destroy-flush, if they are
  not already covered indirectly.

Out of scope
- Rewriting `store.svelte.ts` to fully adopt the primitive if that risks the
  idempotent-bootstrap behaviour; if you do refactor it, preserve that behaviour and test it.

Verify
  bun run check && bun run lint && bun run test:unit
Update AGENTS.md / src/AGENTS.md with the new primitive and the rule for when to use it.
```

---

## PROMPT P4 — Consolidate persistence channels (retire sidebar `localStorage`)

```text
Goal
There are two persistence channels with no written rule: backend SQLite `settings` for app
settings and per-component UI state, and `localStorage` for sidebar state. Pick one rule,
document it, and migrate the outlier.

Current localStorage usage (src/screens/sidebar/Sidebar.svelte)
- `sidebar_tab_top`   (tab position, float)
- `sidebar_notes`     (notes text)
- `sidebar_show_notes`(notes pane open/closed)
(The only other localStorage use is the dev inspector at src/routes/dev/+page.svelte — leave it.)

Required outcome
- A written rule in src/AGENTS.md: backend `settings` is the source of truth for anything that
  should survive a webview data reset / be shared across windows; `localStorage` is not used
  for user-facing state.
- Migrate the three sidebar values to the backend settings API introduced by P1, using the
  P3 primitive. Keep the draggable tab behaviour identical.
- On first run after migration, seed from any existing `localStorage` values so current users
  do not lose their sidebar position/notes; then stop writing to `localStorage`.
- Note: `notesText` is free-form text and is written on every keystroke today. Use the
  debounced write-back from the primitive; do not write per keystroke.

Constraints
- This window is a separate Tauri webview (`label === 'sidebar'`); the value must be readable
  from that window via the shared backend, not from window-scoped storage.
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

## PROMPT P5 — Command-layer test harness + smoke tests

```text
Goal
37 `#[tauri::command]` functions exist and none are tested against an `AppHandle` or a mock
runtime. A typo in `generate_handler!` or a JS/Rust argument-casing mismatch is invisible to
`cargo nextest` until runtime. Give the command layer a paved road.

Read first
- src-tauri/src/lib.rs (`generate_handler!`, `db_pool`)
- src-tauri/src/commands/*.rs, src-tauri/src/settings.rs, src-tauri/src/screens/plan/mod.rs
- src-tauri/src/db/mod.rs (tests use `init_db("sqlite::memory:")`)
- src-tauri/Cargo.toml (note: `tauri` has no `test` feature enabled yet)

Required outcome
- A documented testing convention in src-tauri/AGENTS.md: commands are thin wrappers over
  `async fn(&SqlitePool, ...)` functions living in `db::` / a testable layer; the command
  body does arg marshalling + `db_pool(&app)` only. Extract bodies where that is not yet true.
- A reusable test helper that builds an in-memory database pool and a mock Tauri app, so a
  command can be invoked end-to-end. Use `tauri::test::mock_builder` / `mock_context`
  (enable the `tauri` `test` feature for tests only — e.g. a `[dev-dependencies]` entry or a
  `test-support` feature; do not enable it for release builds).
- If a full mock-webview round trip proves impractical on this platform, fall back to:
  (a) extracting the command body into an `&SqlitePool` function and testing that, and
  (b) a minimal registry test asserting every command name in `generate_handler!` is unique
  and snake_case, plus a test that the JS-facing argument names match the Rust parameters.
  Say clearly in AGENTS.md which approach is used and why.

Smoke tests to add (at minimum)
- `settings::get_setting` / `settings::update_setting` round trip through the real handler.
- One `commands::tasks` command and one `commands::events` command round trip.
- `get_settings` returns defaults when the table is empty.

Constraints
- Do not change production behaviour to make tests easier, except the "thin wrapper"
  extraction, which must be behaviour-preserving.
- Keep test modules `#![allow(clippy::unwrap_used, clippy::expect_used)]` and otherwise clean
  under `cargo clippy --all-targets -- -D warnings`.
- Depends on P1 + P2 merged (test the settled command surface).
- If you refactor `settings.rs` or `screens/plan/mod.rs`, coordinate so P3 (frontend-only) is
  not running concurrently.

Verify
  cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo nextest run
  git diff --exit-code src/lib/bindings
Update src-tauri/AGENTS.md with the command-testing convention.
```

---

## PROMPT P9 — Independent verification (read-only)

```text
Role
You are an independent verifier. You did not write any of these changes. Be adversarial.
Do not fix anything; produce a findings report.

Scope
Review the merged result of P1–P5 against their stated acceptance criteria and the repo rules
in AGENTS.md / src/AGENTS.md / src-tauri/AGENTS.md.

Specifically check
1. Settings substrate (P1)
   - Does `get_settings` still silently drop values it cannot merge? Prove it with a test or a
     concrete counterexample.
   - Is the "what belongs in settings vs ui_state" rule actually documented and followed?
   - Any second JSON-decode rule left in `settings.rs`?
2. Schema single source of truth (P2)
   - Is there a test that fails when a new `AppSettings` field is added but not described?
     If not, the drift protection is not real. Say so.
   - Do setting ids / labels / option values / defaults still match the pre-refactor output?
3. Hydration primitive (P3)
   - Are the three original copies actually unified, or is `persistState` still carrying its
     own duplicate state machine?
   - Is `store.svelte.ts` still idempotent (one cached bootstrap promise) if it was touched?
4. Persistence channels (P4)
   - Is there any remaining `localStorage` write for user-facing state?
   - Does the one-time migration seed correctly and then stop consulting localStorage?
5. Command tests (P5)
   - If a smoke test was skipped in favour of a registry test, is the documented reason
     honest, or is it hiding an untested command surface? A green suite must not be presented
     as more coverage than it is.

Also run the full gate yourself and report the raw result:
  bun run check && bun run lint && bun run test:unit
  cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo nextest run
  git diff --exit-code src/lib/bindings
  bun run check:deps

Output
- Ordered findings: blocking / should-fix / nit.
- For each: file:line, why it violates the stated criterion, and the smallest repro or test
  that demonstrates it.
- Explicitly list which acceptance criteria you could NOT verify, and why.
- Do not propose broad rewrites; propose the minimal fix per finding.
```

---

## 3. Merge / gate checklist

After each merge:

- [ ] `git diff --exit-code src/lib/bindings` is clean (bindings committed).
- [ ] `bun run check`, `bun run lint`, `bun run test:unit` pass.
- [ ] `cargo clippy --all-targets -- -D warnings` and `cargo nextest run` pass.
- [ ] `AGENTS.md` reflects any new file, table, command, or convention.
- [ ] No prompt renegotiated its scope silently — if an agent deviated, the log says why.

Stop-and-report triggers (any agent):

- A dependency or module the prompt assumes does not exist.
- The "correct" fix requires changing a public command name or the settings-screen contract.
- A test would have to be deleted or weakened to make the change pass.
- Anything with user-visible data-loss risk (settings, notes, sidebar position).
