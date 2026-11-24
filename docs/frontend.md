# Frontend reference

Read [frontend instructions](../src/AGENTS.md) for the mandatory conventions.
This document maps shared behavior to its implementation rather than prescribing
individual control labels, colours or dimensions.

## Backend-backed state

`src/lib/store.svelte.ts` wraps calendar mutations. A successful command finishes
the server write before `STORE_UPDATED` is emitted and live queries re-fetch;
components do not optimistically patch calendar arrays. `store.mutate(operation,
action)` reports failures through `notifyError` in `src/lib/toast.ts`.
`store.error` is reserved for fatal bootstrap/load failures. The copyable error
toast is mounted in each window except the effect overlay.

`safeInvoke.svelte.ts` supplies `ResultAsync`, `useTauriQuery` and `useAutoQuery`.
`asyncState.svelte.ts` supplies stale-response guards, flushable/cancellable
debouncers and effect-based hydration (`hydrateOnce` can rerun when its loader
reads reactive dependencies). `persistState` uses fixed-key hydration to read
`ui-state.json` once, suppress writes during hydration and debounce changed values,
flushing pending writes on destroy. Module-level singleton bootstrap is a different
lifecycle: `AppStore.init()` caches its bootstrap promise and briefly retries
`not-ready` settings reads while Rust setup finishes.

`taskList.svelte.ts` is the shared filter/sort definition for the plan task pane,
Do screen and mini tracker. `TaskListState.svelte` persists `ui.task_list.*` from
the layout; the mini tracker reloads the definition when opening its menu, then
refreshes its list query asynchronously. This is not a cross-window display barrier.
Sidebar state uses `ui.sidebar.*` for notes, visibility and tab position.

## Calendar rendering and dates

- Raw `AppEvent` is for editing; render surfaces use projected `EventInstance`s.
  `domain/timing.ts` owns `instanceOccursOnDay` and `taskDateOccursOnDay`.
  All-day dates float; timed instances compare against local day boundaries, so
  DST days need not be 24 hours long.
- `occurrenceKey` is the stable rendering identity; `occurrenceStart` is the
  canonical slot sent back to scoped mutations. Use `domain/events.ts`'s
  `isRecurring` for raw-row recurrence detection, including nullable wire fields.
- `date-grid/dates.ts` builds cells and the inclusive query range together.
  Month navigation uses `time.ts`'s month-safe `addMonths`.
- `spanLayout.ts` clips multi-day all-day events to each week and packs lanes;
  they render once per week, not once in each day cell. Single-day/timed events
  stay inline. `eventColor.ts` supplies the shared grid/timeline accents.
- Task `start` and `due` produce separate non-draggable grid markers, even on the
  same day. The marker body opens the inspector; its circle toggles completion.
  Markers do not reschedule dates/events; dragging a task-list row is separate.

`eventVisibility.ts` applies grid-only presentation choices to **projected**
instances before both inline bucketing and span layout. `hideTimed` reads
`timing.kind`; `hideFrequentRecurring` tests the inherited rule's `FREQ` against
`SECONDLY`, `MINUTELY`, `HOURLY`, `DAILY`, `WEEKLY`, ignoring `INTERVAL`.
Thus “frequent” is a frequency-category label, not a measured interval: a
`FREQ=WEEKLY;INTERVAL=8` series is also hidden. Monthly/yearly series remain visible.
Both flags default off, persist under `ui.date_grid.visibility`, and leave task
markers alone. Do not filter masters/exceptions before backend expansion.

## Pointer gestures and timeline geometry

Planning paths here are under `src/screens/plan/`:

- `day-timeline/hooks/pointerGesture.svelte.ts` registers window pointer listeners
  with cancellation/teardown safety. Event/task drag hooks share the pointer
  pipeline and hit-test `data-drop-kind` targets (`grid-day`, `day-time`).
- `hooks/eventDrag.svelte.ts` delegates payloads to `date-grid/dropPayload.ts`.
  Grid drops shift the day, preserving timed local time/duration. Timeline drops
  use a snapped minute; an all-day instance defaults to a one-hour timed block.
  Recurring drags ask for `this` or `all`; `following` moves are unsupported.
- `hooks/taskDrag.svelte.ts`: a task-row grid drop sets date-only `start`; a
  timeline drop creates a linked event, using the estimate or default duration.
- `snapping.ts` owns snapping/duration clamping. `day-timeline.css` owns horizontal
  geometry: inherited `--day-label-offset`, event lane variables and a 24px
  pointer-creation gutter. Grid lines and previews share that geometry; event
  borders are left accents only.

Sort-menu row reordering uses native HTML5 drag events instead of that pointer
pipeline. Keep Tauri `dragDropEnabled: false`: the native handler interferes with
in-page drag/drop on Windows. This is not a requirement to enable native file drops.

## Filters and inspector

`FilterButton`/`SortButton` share `FloatingMenu`; filter conditions and sortable
rows share `FilterSortRow`. `FilterGroupEditor` recursively edits independent
AND/OR groups: the first child is labelled “Where”, the second child's connector
selects AND/OR for the whole group, and later connectors display that operator.
New groups default to AND; groups with fewer than two children have no operator
control. Clear/remove-group actions sit alongside the add actions at the bottom.
Badges count conditions, not groups. `EventFilterButton` provides the common
calendar configuration with optional grid-only visibility controls. Tags use the
backend `list_tags` multi-select; date conditions support comparison operators and
`today`/`tomorrow`/`yesterday` as well as date literals. Every column also offers
`is empty`/`is not empty`, which hide the value control and clear stale values.

`domain/filterTree.ts` owns construction/counting and task/event persistence
validation. The four filter-state keys reject malformed trees/old arrays without
migration. `domain/filters.ts` derives new-task defaults only from required
conditions, intersecting shared constraints across OR branches. Value-free
conditions are effective branches but do not supply literal defaults, so an
“empty tags OR tags is work” filter does not assign `work` to new tasks. Backend
semantics are documented in the [calendar feature](../src-tauri/src/features/calendar/README.md).

The inspector's `state.svelte.ts` owns selection, queries, mutations and recurring
scope prompts. `eventDates.ts` handles inclusive all-day ends and timed/all-day
conversion; `recurrence.ts` owns presets. Event-to-task conversion copies title,
description, tags, priority and start into the default writable task list, then
removes the event under the selected scope. `domain/events.ts`'s `buildNewEvent`
is the shared constructor for new-event drafts.

## Quick capture

`src/lib/launcher/LauncherPalette.svelte` queries today's agenda when empty and
uses debounced `parse_quick_capture` for typed previews. Sigil/date parsing and
writable-list resolution stay in Rust. Enter on the create row builds an `AppTask`
and calls the existing `create_task` path; agenda rows navigate instead. Route
words emit `LAUNCHER_NAVIGATE` to the main window, not the launcher.

Currently the agenda range is captured on mount, so a retained launcher webview
can show the previous day's agenda after midnight. This is a
[follow-up bug](known-limitations.md), not behavior to preserve.
