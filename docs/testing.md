# Testing reference

Test behavior and interoperability, not a transcription of implementation details.
Read the [test-review skill](../.agents/skills/avoid-change-detector-tests/SKILL.md)
when writing or reviewing tests. Do not add coverage thresholds.

## Commands

From the repository root:

```sh
bun run check
bun run lint
bun run test:unit
bun run test                 # Playwright; boots the Vite dev server
bun run coverage:frontend
```

`bun run check` uses `svelte-check --tsgo`; `bun run lint` runs Oxlint then
ESLint. `package.json` and `.github/workflows/ci.yml` own the exact invocations.

From `src-tauri/`:

```sh
cargo clippy --all-targets -- -D warnings
cargo nextest run            # also exports ts-rs bindings and docs/cli.md
```

`bun run coverage:backend` runs `cargo llvm-cov nextest` (requires
`cargo-llvm-cov`). CI uses frontend coverage and backend llvm-cov/nextest, then
checks tracked generated-binding and CLI-reference drift. New generated files must
also be added to version control—the diff gate alone does not detect untracked
outputs. See [Windows development](windows-development.md) if temp-directory
access fails; that workaround applies to backend tests too.

## Vitest and Svelte

Tests live next to code as `*.test.ts`; component tests use
`@testing-library/svelte` with jsdom. Shared utilities are in `src/test/`.

- `makeEvent`/`makeInstance` from `factories.ts` supply domain fixtures with partial
  overrides, including nested timing.
- `vite.config.js` selects Svelte's browser build for Vitest. Removing that
  condition can resolve the server build and prevent component mounting.
- `setup.ts` installs Tauri runtime/event globals used by the externalized event
  API. Reuse it rather than inventing globals in each test. These no-op globals
  are scaffolding, not an event-delivery simulation; event behavior needs an
  explicit event mock.
- `tauriMock.ts` shares the IPC mock between the test and its module factory
  within each isolated test environment. Keep the factory top-level so Vitest
  hoists it; register commands and reset in `beforeEach`:

```ts
vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();
// beforeEach: tauri.reset();
// Register: tauri.stubCommand(cmd, handler) / stubCommandValue(cmd, value).
// Assert calls through tauri.invoke. Adjust the import path for the test location.
```

Unregistered commands resolve to `undefined`. Vitest globals are disabled, so
Testing Library's automatic cleanup does not register: import `afterEach` from
Vitest and explicitly call `cleanup()` in component tests. A small `.svelte`
harness is appropriate for rune/lifecycle behavior that cannot run from plain TS;
prefer mocking the backend over mounting an entire screen.

`tests/e2e/day-timeline.spec.ts` checks stylesheet geometry with DOM fixtures,
bounding boxes and hit-testing. It neither captures screenshots nor starts the
native desktop app.

## Rust behavior tests

Write tests first for backend behavior. Existing coverage includes CalDAV XML
parsers, ICS preservation invariants, store/query behavior, settings and error
shapes. Reuse `test_support::caldav::{start_server, start_http_server}` and
`test_support::ics::{series, all_day, with_exception}` for server/ICS fixtures.
`clippy.toml` permits `unwrap`/`expect` in tests; do not add repetitive module allows.

For real multi-account behavior there is a disposable loopback Radicale fixture
with documented test-only credentials; see [dummy accounts](dummy-accounts.md).
Its backend contract is `caldav/tests/dummy_accounts.rs`, ignored by default and
run through the fixture script (`--check-backend`), like the Radicale e2e example:

The live Radicale check is `src-tauri/examples/e2e_caldav.rs`. It exercises
create/edit/move/delete and preservation through a real server, is not part of CI,
and needs deliberate access to a **disposable data directory and server fixture**:
it recursively deletes the selected data directory and edits/restores a server
task fixture. Never point this
example at the user's normal data directory/account as a harmless check:

```sh
TASKROOT_DATA_DIR=target/e2e-data cargo run --example e2e_caldav -- --user … --password …
```

## IPC source contract: what it does and does not prove

`ipc/tests.rs` checks registration uniqueness, agreement between registered and
annotated handlers, literal frontend command names and recognized supplied keys
against camelCased Rust parameter names. `test_support/source_scan.rs` parses Rust;
`frontend_scan.rs` parses frontend call sites, including later `query.execute({…})`
for a locally bound query. Test sources are excluded.

These are shallow source scans, **not end-to-end handler tests or a fully typed
IPC contract**:

- Computed command names are not checked.
- Argument builders, non-literal objects and spread members are not resolved.
  For example, `query.execute(allTasks())` exposes no argument keys to this scan.
- Only recognized supplied keys are checked; missing required arguments are not.
- Rust signature parsing splits on commas; generic types containing commas can
  confuse it. Handler-table parsing stops at the first closing `]`.
- Query-binding matching is text-based, not scope-aware. A green run does not
  exercise handler state/client resolution, networking or event emission.

Current handlers use the default concrete `AppHandle` runtime. Tauri's mock IPC
entry point requires `MockRuntime`-compatible handlers, so the production table
cannot simply be installed into a mock app unchanged. Tauri's `test` feature is
not enabled here today; a future mock suite must also enable it and adapt the
relevant helpers, or use a smaller compatible table. This is a constraint of
this test setup, not proof that runtime-level testing is impossible. Behavior
belongs below the command seam; pure adapter decisions can also be tested directly.
Source checks supplement those tests.

If extending the wire surface, review scanner coverage explicitly rather than
assuming a new wrapper/builder is followed automatically. Do not genericize all
production commands solely to work around this test harness.
