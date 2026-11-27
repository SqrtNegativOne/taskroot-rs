# Frontend instructions

Supplements [the root guide](../AGENTS.md) for `src/`. Read
[frontend](../docs/frontend.md), [planning UI](../docs/planning-ui.md),
[themes](../docs/themes.md) or [testing](../docs/testing.md) as needed for the
affected area.

## Svelte and TypeScript

- Use Svelte 5 runes (`$state`, `$derived`, `$effect`, `$props`), never legacy
  `$:` or `export let`.
- Wrap intentional one-time prop reads that seed state in `untrack`, e.g.
  `$state(untrack(() => defaultSize))`; do not suppress
  `state_referenced_locally`.
- Preserve end-to-end types: no `any`, `as unknown as` casts or loose records as
  compiler escapes.
- Use `Routes` from `src/lib/routes.ts` for route paths.
- Keep the three pointer-UI a11y warning filters in `svelte.config.js`
  (`a11y_autofocus`, `a11y_click_events_have_key_events`,
  `a11y_no_static_element_interactions`), not per-node `svelte-ignore` comments.

## State, queries and errors

- Use `safeInvoke`/`useTauriQuery`/`useAutoQuery` (`lib/safeInvoke.svelte.ts`)
  and `neverthrow` for expected IPC failures, not `try/catch`.
- Wrap user-triggered calendar mutations in `store.mutate(operation, action)`.
  Recoverable errors go to `notifyError` toasts; `store.error` is for fatal
  bootstrap/load failures only.
- Await backend writes and let `STORE_UPDATED` trigger re-queries. Never
  optimistically patch local calendar arrays or cache them in `localStorage`.
- Use `update_setting` for typed settings, `persistState` for component UI
  state, `lib/asyncState.svelte.ts` for stale guards/debouncers/hydration, and
  `lib/useNow.svelte.ts` for reactive clocks.
- Reuse argument builders in `lib/queries.ts` (they standardize argument shapes,
  not command names) and the shared task-list state in `lib/taskList.svelte.ts`.

## Domain and UI invariants

- Never derive calendar days with `Date.toISOString()`; use `lib/time.ts`
  (`ymd`, `addMonths`, `parseLocalYmd`) and `lib/domain/timing.ts`.
- Key rendered events by `occurrenceKey`; pass `occurrenceStart` on scoped
  mutations; never compose `RRULE`/`EXDATE`/ICS text or run quick-capture
  parsing on the frontend.
- Keep recursive filter validation in `lib/domain/filterTree.ts` and new-task
  default inference in `lib/domain/filters.ts`.
- Use semantic CSS variables from `lib/themes/`; keep today indicators on
  `--today*` tokens and keep server-owned calendar colours separate.

Run the frontend checks in [AGENTS.md](../AGENTS.md#required-checks).
