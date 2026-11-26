# Frontend instructions

These rules supplement [the root guide](../AGENTS.md) for `src/`.
Read [the frontend reference](../docs/frontend.md) for the affected feature and
[testing setup](../docs/testing.md) when adding/reviewing tests.

## Svelte and TypeScript

- Use Svelte 5 runes (`$state`, `$derived`, `$effect`, `$props`), not legacy `$:`
  or `export let` reactivity.
- Wrap intentional one-time prop reads that seed state in `untrack`, e.g.
  `$state(untrack(() => defaultSize))`. Do not suppress
  `state_referenced_locally` to hide the issue.
- Preserve end-to-end types; do not use `any`, casts through `unknown` or loose
  records as lint/compiler escapes. Refactor generic components properly when
  needed rather than patching away type safety.
- Use `Routes` from `src/lib/routes.ts` for route paths.
- The three pointer-UI a11y warning filters belong in `svelte.config.js`
  (`a11y_autofocus`, `a11y_click_events_have_key_events`,
  `a11y_no_static_element_interactions`), not per-node `svelte-ignore` comments.

## State, queries and errors

- Use `safeInvoke`/`useTauriQuery`/`useAutoQuery` from `lib/safeInvoke.svelte.ts` and
  `neverthrow` for expected IPC failures, not ordinary `try/catch`.
- Wrap user-triggered calendar mutations in `store.mutate(operation, action)`.
  Recoverable failures go through `notifyError`/copyable toasts; `store.error`
  is for fatal load/bootstrap failures only.
- Await backend writes, then let `STORE_UPDATED` trigger re-querying. Do not
  optimistically patch local calendar arrays or cache them in localStorage.
- App settings use `update_setting`/`settings.json`; component state uses
  `persistState`/`ui-state.json`. Reuse the stale guard, debouncer and hydration
  primitives in `lib/asyncState.svelte.ts` instead of duplicating timer/request
  state machines. Use `lib/useNow.svelte.ts` instead of ad-hoc clock loops.
- Reuse `lib/queries.ts` argument builders. They centralize argument shapes,
  **not command names**, and are not fully followed by the IPC source scan.
- Shared task-list filters/sorts live in `lib/taskList.svelte.ts`; do not create
  separate definitions for the plan pane, Do screen or mini tracker.

## Calendar and interaction invariants

- Day bucketing/arithmetic uses `lib/time.ts`. Never derive a calendar day with
  `Date.toISOString()` (it shifts to UTC); use month-safe `addMonths` navigation.
- Render events/tasks through `EventInstance`/`TaskDateInstance` and
  `lib/domain/timing.ts`'s day-membership helpers. All-day dates float; timed
  instances use local day boundaries. Raw `AppEvent` rows are for editing.
- Key events by `occurrenceKey`; send `occurrenceStart` with scoped mutations.
  Use `lib/domain/events.ts`'s `isRecurring` and `buildNewEvent`. Never construct
  `EXDATE`/`RRULE`/ICS text in components.
- Grid visibility filters operate on projected instances before bucketing **and**
  span layout, never on raw recurrence context. Task date markers are
  non-draggable but allow completion toggles; task-list-row dragging is separate.
- Application colours use semantic CSS variables from `lib/themes/`; palettes
  own base colours, and `tokens.css` owns shared derived tints. See
  [colour schemes](../docs/themes.md). Today/current-time indicators use the
  shared red `--today` tokens, not the theme accent. Keep server-owned calendar
  colours separate.
- Reuse the shared event colors, timeline CSS geometry, snapping/duration helpers
  and pointer gesture registry; retain cancellation/teardown safety.
- Keep `dragDropEnabled: false` in Tauri config for in-page HTML5 sort-row drags
  on Windows. It is separate from planning's pointer-based drag pipeline.
- Recursive filter construction/validation belongs in `lib/domain/filterTree.ts`;
  new-task default inference belongs in `lib/domain/filters.ts`. Invalid persisted
  filter trees fall back to defaults without migration.
- Launcher sigil/date parsing and recurring mutation scope stay in Rust.

Run the frontend checks in the [root guide](../AGENTS.md#required-checks).
For component tests, retain the shared Tauri mock/globals, Vitest browser
resolution and explicit cleanup described in [testing](../docs/testing.md).
