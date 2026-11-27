# Frontend reference

Read [frontend instructions](../src/AGENTS.md) for mandatory conventions. For
the Plan screen (date grid, visibility toggles, pointer drag/drop and timeline
geometry), see [planning UI](planning-ui.md); for palettes and tokens, see
[colour schemes](themes.md).

## Backend-backed state

- `src/lib/store.svelte.ts` wraps calendar mutations. Successful commands finish
  the server write before `STORE_UPDATED` triggers live re-queries; components
  never optimistically patch calendar arrays. `store.mutate(operation, action)`
  reports recoverable failures through `notifyError` (`src/lib/toast.ts`,
  mounted in every window except `effect`); `store.error` is reserved for fatal
  bootstrap/load failures.
- `src/lib/safeInvoke.svelte.ts` provides `ResultAsync`, `useTauriQuery` and
  `useAutoQuery`. Queries re-fetch on `STORE_UPDATED` (including after account
  commands). `AccountBoundary` re-keys non-Settings routes on `account-changed`
  to discard cross-account drafts.
- `src/lib/asyncState.svelte.ts` supplies stale-response guards, flushable
  debouncers and effect-based hydration (`hydrateOnce` reruns if its loader
  reads reactive dependencies). `persistState` (`src/lib/persisted.svelte.ts`)
  hydrates `ui-state.json` once, suppresses writes during hydration, debounces
  changes and flushes on destroy.
- `AppStore.init()` caches its bootstrap promise and briefly retries `not-ready`
  settings reads while Rust setup finishes.
- `src/lib/taskList.svelte.ts` owns the shared filter/sort definition used by
  the plan pane, Do screen and mini tracker. `TaskListState.svelte` persists
  `ui.task_list.*` from the layout; the mini tracker reloads it when opening its
  menu and refreshes asynchronously. Sidebar state persists under `ui.sidebar.*`.

## Filters and inspector

- `FilterButton` shares `FloatingMenu`; filter conditions and sort rows share
  `FilterSortRow`. `ViewMenuButton` combines a filter editor and HTML5-draggable
  sort list in one popover for the task list header.
- `FilterGroupEditor` recursively edits independent AND/OR groups: the first
  child is labelled "Where", the second chooses AND/OR for the group, and later
  connectors display it. New groups default to AND; badges count conditions, not
  groups. `EventFilterButton` adds optional grid-only visibility toggles.
- Tag pickers read backend `list_tags`; date conditions accept comparisons,
  relative keywords (`today`/`tomorrow`/`yesterday`) and literals; every column
  supports value-free `is empty` / `is not empty`.
- `src/lib/domain/filterTree.ts` validates persisted filter trees (rejecting
  malformed trees without migration). `src/lib/domain/filters.ts` infers new-task
  defaults only from required conditions, intersecting shared constraints across
  OR branches (value-free conditions are effective branches that supply no
  literal default). Backend filter evaluation is documented in
  [calendar feature](../src-tauri/src/features/calendar/README.md).
- `src/components/inspector-pane/state.svelte.ts` owns selection, queries,
  mutations and recurring scope prompts; `eventDates.ts` handles inclusive
  all-day ends and timed/all-day conversion; `recurrence.ts` owns presets.
  Event-to-task conversion copies title, description, tags, priority and start
  into the default writable task list, then deletes the event under the chosen
  scope.

## Quick capture

`src/lib/launcher/LauncherPalette.svelte` queries today's agenda when empty
(refreshing the local day reactively on focus, visibility change and a 60 s
clock tick) and debounces `parse_quick_capture` for typed previews. Sigil/date
parsing and writable-list resolution stay in Rust. Pressing Enter on the create
row calls `create_task`; agenda rows navigate to Plan; route words emit
`LAUNCHER_NAVIGATE` to the main window.
