# Planning UI

Date-grid rendering, visibility toggles, pointer drag/drop and day-timeline
geometry under `src/screens/plan/` and `src/lib/`.

## Calendar rendering and dates

- Render surfaces consume projected `EventInstance` and `TaskDateInstance`
  values; raw `AppEvent` rows are for editing.
- `src/lib/domain/timing.ts` owns `instanceOccursOnDay` and
  `taskDateOccursOnDay`. All-day dates float; timed instances compare against
  local day boundaries so DST days need not be 24 hours long.
- `occurrenceKey` is the stable render key; `occurrenceStart` is the canonical
  slot sent back to scoped mutations. Use `isRecurring` and `buildNewEvent` from
  `src/lib/domain/events.ts`.
- `date-grid/dates.ts` builds week slices, dominant-month viewport scoring and
  inclusive query windows; `date-grid/infiniteGrid.svelte.ts` drives vertical
  infinite scrolling (scrollbar hidden) across Month, 3-week and 1-week views
  with live dominant-month date highlighting, header updates and smooth week-row
  snapping (`date-grid/weekSnap.ts`). Month navigation uses `src/lib/time.ts`'s
  month-safe `addMonths` (never `Date.toISOString()`).
- `date-grid/spanLayout.ts` clips multi-day all-day events per week and packs
  lanes so they render once per week; single-day and timed events render inline.
- `src/lib/eventColor.ts` parses server `#RRGGBB`/`#RRGGBBAA` colours into fill,
  text and accent bar tokens; `src/lib/themes/eventTheme.svelte.ts` recomputes
  them when the palette changes. See [colour schemes](themes.md#server-owned-calendar-colours).
- Task `start` and `due` produce separate grid markers, even on the same day.
  When the task's calendar is writable, markers are draggable onto another grid
  day (moving `start` or `due` while preserving time of day) or onto the
  timeline (scheduling a linked event); read-only markers are non-draggable.
  Clicking the body opens the inspector; clicking the circle toggles completion.

## Grid visibility filters

`src/lib/eventVisibility.ts` filters **projected** instances before both inline
bucketing and span layout (never filter raw masters/exceptions before backend
expansion). All three flags default `false` and persist under
`ui.date_grid.visibility`:

- `hideTimed` — checks `timing.kind === 'timed'`.
- `hideFrequentRecurring` — matches `FREQ` in `SECONDLY`, `MINUTELY`, `HOURLY`,
  `DAILY`, `WEEKLY`, ignoring `INTERVAL` (e.g., `FREQ=WEEKLY;INTERVAL=8` is also
  hidden; monthly and yearly stay visible).
- `hideDoneTasks` — hides planned events linked to a `done` task and drops both
  `start` and `due` markers of completed tasks.

## Pointer gestures and timeline geometry

Paths below are under `src/screens/plan/`:

- `day-timeline/hooks/pointerGesture.svelte.ts` registers window pointer
  listeners with cancellation/unmount teardown safety. Event and task drag hooks
  share this pipeline and hit-test `data-drop-kind` targets (`grid-day`,
  `day-time`).
- `hooks/eventDrag.svelte.ts` delegates drop calculation to
  `date-grid/dropPayload.ts`: grid drops shift the day while keeping timed local
  time/duration; timeline drops snap to minutes (all-day becomes a 1-hour timed
  block). Recurring drags prompt for `this` or `all` (`following` moves are
  unsupported).
- `hooks/taskDrag.svelte.ts`: dropping a task row on a date-grid cell sets
  date-only `start`, while dropping a date-grid task marker on another cell
  moves its `start` or `due` date (preserving any time of day); dropping either
  on the timeline creates a linked `VEVENT` (`X-TASKROOT-TASK-ID`) using the
  task's estimate or the Default Duration setting.
- `snapping.ts` owns snapping, duration clamping, and `earliest_wake_up_time` start-hour resolution (`resolveStartHour`, `snappedDropMinuteFrom`). `DayTimeline` hides hours before `startHour`, exposing `data-start-minute` on `.day-grid` so pointer creation, drops, event blocks, previews, and the current-time indicator align with the visible hour window while preserving real day timestamps. `day-timeline/infiniteTimeline.svelte.ts`
  drives horizontal infinite scrolling over a buffered sliding day window,
  sizing each day column from the viewport width divided by the chosen visible
  day count (`numDays`) while keeping the hour gutter sticky; `day-timeline/scrollSnap.ts`
  smoothly aligns horizontal scroll gestures onto day boundaries. `day-timeline.css`
  owns the shared header/day grid tracks and hour gutter, importing
  `timeline-events.css` (event lanes and the 24 px creation strip) and
  `timeline-controls.css` (eye menu). Previews share day-grid geometry; event
  borders are left accents only.
- Sort-row reordering in view menus uses native HTML5 drag events instead of the
  pointer pipeline. Keep `dragDropEnabled: false` in `src-tauri/tauri.conf.json`
  so Tauri's native handler does not intercept in-page drags on Windows.
