# Testing reference

Test behavior and interoperability, not copies of the implementation; see
[avoid-change-detector-tests](../.agents/skills/avoid-change-detector-tests/SKILL.md).
Do not add coverage thresholds.

## Commands

From the repository root:

```sh
bun run check              # svelte-check --tsgo
bun run lint               # Oxlint then ESLint
bun run test:unit          # Vitest (jsdom)
bun run test               # Playwright (boots Vite dev server)
bun run coverage:frontend
```

From `src-tauri/`:

```sh
cargo clippy --all-targets -- -D warnings
cargo nextest run          # also regenerates src/lib/bindings/ and docs/cli.md
```

`bun run coverage:backend` runs `cargo llvm-cov nextest`. CI runs coverage and
checks tracked drift on `src/lib/bindings/` and `docs/cli.md` (new generated
files must still be `git add`ed). If temp-directory access fails on Windows, see
[temporary directories](windows-development.md#temporary-directories).

## Vitest, Svelte and Playwright

Unit/component tests live beside code as `*.test.ts` using
`@testing-library/svelte` + jsdom. Shared test utilities live in `src/test/`:

- `factories.ts`: `makeEvent` / `makeInstance` fixtures with partial overrides.
- `vite.config.js`: sets `conditions: ['browser']` under Vitest so Svelte
  resolves its browser build and components can mount.
- `setup.ts`: enables automatic Testing Library DOM cleanup
  (`@testing-library/svelte/vitest`) and installs no-op Tauri runtime/event
  globals (mock events explicitly when testing event delivery).
- `tauriMock.ts`: shared per-environment IPC mock. Keep the `vi.mock` factory
  top-level so Vitest hoists it, then reset and stub commands in `beforeEach`:

```ts
vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();
// beforeEach: tauri.reset();
// Register: tauri.stubCommand(cmd, handler) / stubCommandValue(cmd, value).
```

Unregistered commands resolve to `undefined`. Prefer small `.svelte` test
harnesses over mounting whole screens.

Playwright specs (`tests/e2e/day-timeline.spec.ts` for CSS geometry/hit-testing,
`themes.spec.ts` for contrast and palette switching) are non-visual browser
checks — they do not take screenshots or launch the Tauri binary.

## Rust behavior tests

Write backend tests first. Reuse `test_support::caldav::{start_server, start_http_server}`
and `test_support::ics::{series, all_day, with_exception}`. `clippy.toml` allows
`unwrap`/`expect` in tests; do not add per-module allows.

- Multi-account integration: disposable loopback Radicale in
  [dummy accounts](dummy-accounts.md) (`caldav/tests/dummy_accounts.rs`, run via
  `--check-backend`).
- Live Radicale smoke test: `src-tauri/examples/e2e_caldav.rs` (not in CI).
  **Destructive**: recursively deletes `TASKROOT_DATA_DIR` and edits a server
  task. Run only against a disposable directory and server fixture:

```sh
TASKROOT_DATA_DIR=target/e2e-data cargo run --example e2e_caldav -- --user … --password …
```

## IPC bindings and contracts

`src-tauri/src/ipc/mod.rs` registers all `#[tauri::command]` + `#[specta::specta]`
handlers and typed `tauri_specta::Event` structs once in `specta_builder()`.
`src-tauri/tests/export_bindings.rs` exports `src/lib/bindings/index.generated.ts`
during `cargo nextest run`, and `bun run check` enforces command names,
argument shapes, return types, and event payloads end-to-end.

Handlers still use concrete `AppHandle` (Tauri's `test` feature / `MockRuntime`
is not enabled), so keep behavior testable below the command seam in the store,
ICS, CalDAV, and settings layers.
