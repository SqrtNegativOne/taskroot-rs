# Taskroot: persistence & settings refactor — complete

All planned waves are complete, committed, and independently verified.

- **P1** settings substrate (serde merge + `ui_state`).
- **P2** setting metadata single source (`#[setting(..)]` + `SettingsMeta`).
- **P3** shared backend-hydration primitive (`src/lib/asyncState.svelte.ts`).
- **P4** sidebar state moved off `localStorage` (`src/screens/sidebar/state.svelte.ts`).
- **P5** command-layer test harness (`src-tauri/src/test_support/`).
- **P9** independent verification: full gate green, no blocking findings.

P9's should-fix items and nits were resolved afterwards:

- **F1** the frontend arg-contract scan now follows `let x = useTauriQuery(...)` to
  its later `x.execute({...})` and checks those keys (`commands/tests.rs`).
- **F2** `commands::events::active_calendars(&SqlitePool)` is extracted and
  exercised by `commands/tests.rs`.
- **N1** the store bootstrap rationale is corrected; **N2** user-visible setting
  strings are pinned in `settings/metadata/tests.rs`; **N3** the intentional
  unknown-key `update_setting` behaviour is documented; **N4** the source-scan
  limits are documented in `src-tauri/AGENTS.md`; **N5** the command count is
  corrected to 33.

No remaining work. Recover the original P1–P9 prompts from git history if needed.
