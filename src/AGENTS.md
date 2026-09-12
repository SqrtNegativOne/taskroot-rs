# Frontend Architecture & Rules

This file contains rules specific to the Svelte 5 frontend of Taskroot. It supplements the root `AGENTS.md`.

## Key Concepts
- **State Management**: Frontend state is managed using Svelte 5 Runes (`$state`, `$derived`, `$effect`, `$props`). The primary store is located in `src/lib/store.svelte.ts`, which syncs with the Rust backend via Tauri IPC (`invoke` wrapped in `safeInvoke`).
- **Persisted UI State**: Per-component UI state (filters, sorts, date-grid view, timeline day count, split-pane sizes, Do-screen section collapse) is persisted with the `persistState` rune (`src/lib/persisted.svelte.ts`) in the backend `ui_state` table via `set_ui_state`. It hydrates once from `get_ui_state` and debounce-saves via `set_ui_state`; while hydration is in flight, writes are suppressed, and only values that differ from the stored/fallback value are written.
- **Persistence Channels**: Backend SQLite is the source of truth for anything that must survive a webview data reset or be shared across windows. Typed app settings are `AppSettings` fields written with `update_setting`; arbitrary per-component UI state goes through `get_ui_state`/`set_ui_state` in the `ui_state` table. Never use `localStorage` for user-facing state.
- **SQLite Source of Truth**: Crucially, the frontend relies strictly on the SQLite backend as the source of truth. It avoids complex optimistic patching arrays locally. Mutations return `neverthrow` `Result`s, `await` the backend command, and then instantly re-fetch the raw state from the database. This prevents race conditions and UI pop-backs, as local SQLite queries return in ~1-3ms.
- **Local-Date Rule**: All day bucketing/comparison must go through `src/lib/time.ts` (`ymd`, `addDays`, `dayDiff`, `sameDay`), which operate on local date parts. `Date.toISOString()` is UTC-shifted and must not be used to derive a calendar day.
- **Event Instance Projection**: Render surfaces (`DateGrid`, `DayCell`, `DayTimeline`, `bucketing.ts`, `EventBlock`) consume `EventInstance` (from `query_event_instances`), never raw `AppEvent` wire strings. All-day vs timed is discriminated via `event.timing.kind`; day membership goes through `src/lib/domain/timing.ts` (`instanceOccursOnDay`). All-day events compare floating `YYYY-MM-DD` strings; timed events compare UTC instants against system-local day boundaries. `occurrenceKey` is the per-occurrence identity used for list keys.

## Testing
Unit tests run under Vitest in a `jsdom` environment (`bun run test:unit`); component tests use `@testing-library/svelte`. Tests live next to the code as `*.test.ts` (`src/**/*.{test,spec}.{js,ts}`) and shared test utilities live in `src/test/`.

- **Browser Svelte build**: `vite.config.js` sets `resolve.conditions: ['browser']` when `process.env.VITEST` is set. Without it, Vitest resolves Svelte's server build under the default node condition and mounting a component fails. Do not remove that conditional.
- **Mocking Tauri IPC**: `src/test/tauriMock.ts` exports `mockTauriInvoke()`, the process-wide `invoke` mock shared by the test and the module factory. Keep the `vi.mock('@tauri-apps/api/core', ...)` call at the top level of the test file so Vitest can hoist it, and hand off through the factory:

  ```ts
  vi.mock('@tauri-apps/api/core', async () => {
      const { mockTauriInvoke } = await import('../test/tauriMock');
      return { invoke: mockTauriInvoke().invoke };
  });

  const tauri = mockTauriInvoke();
  ```

  Register behaviour with `tauri.stubCommand(cmd, (args) => ...)` or `tauri.stubCommandValue(cmd, value)`; unregistered commands resolve to `undefined`. Clear state with `tauri.reset()` in `beforeEach`. Assert directly on `tauri.invoke` (e.g. `toHaveBeenCalledWith`).
- **Cleanup**: Vitest is not configured with `globals: true`, so `@testing-library/svelte`'s automatic `afterEach(cleanup)` never registers. Every component test must import `afterEach` from `vitest` and call `cleanup()` explicitly.
- **Component harnesses**: Prefer mocking the backend over mounting a full screen. A tiny harness `.svelte` component (see `src/lib/persistedHarness.svelte`) is warranted when the behaviour under test is driven by a rune or lifecycle hook rather than markup, since a rune cannot be exercised from a plain `.ts` test.
- Do not add coverage thresholds.

## Style & Idioms
- **Svelte 5 Idioms**: Strictly use Svelte 5 runes (`$state`, `$derived`, `$effect`, `$props`) instead of legacy Svelte 4 reactivity (`let foo = ...`, `$:`, `export let`).
- **One-Time Prop Reads**: When a prop is read once to seed local `$state` (e.g. `defaultSize`, `persistKey`, `defaultOpen`), wrap it in `untrack`: `let size = $state(untrack(() => defaultSize))`. This documents the intent and silences Svelte's `state_referenced_locally` warning. Do not use `// svelte-ignore state_referenced_locally` for this.
- **Typescript Idioms**: Strongly type your code. Never use `any`, `unknown` casts, or loose interfaces (like `Record<string, unknown>`) as quick hacks to bypass ESLint, Oxlint, or TypeScript compiler errors. If fixing a type warning requires a larger architectural change—such as utilizing Svelte 5 component `generics="T"` to preserve end-to-end type safety for generic UI components—you must do the thorough refactor rather than applying a bandage solution.
- **Frontend Error Handling (`neverthrow`)**: Use the `neverthrow` library to handle errors functionally on the frontend, mirroring the Rust backend's `Result` type. Do not use standard `try/catch` for expected errors. When calling Tauri's `invoke`, use the `safeInvoke` wrapper (or `useTauriQuery` rune) located in `src/lib/safeInvoke.svelte.ts` to ensure type-safe `ResultAsync` returns.
- **Routing**: Never use magic strings for route paths (e.g., `goto('/login')`). Always import and use the centralized constants from `src/lib/routes.ts` (e.g., `goto(Routes.LOGIN)`).
