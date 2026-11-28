<script lang="ts">
    import { onMount, untrack } from 'svelte';
    import './date-grid.css';
    import { DateGridView, DAYS_IN_WEEK } from './constants';
    import CalendarHeader from './CalendarHeader.svelte';
    import DayCell from './DayCell.svelte';
    import type { AppCalendar, EventInstance, AppEventFilter, FilterGroup, TaskDateInstance } from '../../../lib/domain';
    import { calendarsForComponent, emptyFilterGroup, isEventFilterGroup, instanceOccursOnDay, isEventReadOnly, taskDateOccursOnDay } from '../../../lib/domain';
    import { resolveEventColor } from '../../../lib/eventColor';
    import { eventColorVars } from '../../../lib/themes/eventTheme.svelte';
    import { isMultiDayAllDay, layoutWeekSpans, type WeekSpan } from './spanLayout';
    import { addDays, getWeekNumber, isoWeeksInYear, isoWeekYear, parseLocalYmd, ymd } from '../../../lib/time';
    import { createInfiniteDateGrid } from './infiniteGrid.svelte';
    import { attachWeekSnap } from './weekSnap';
    import { isEventVisible, isEventVisibility, type EventVisibility } from '../../../lib/eventVisibility';
    import { useNow } from '../../../lib/useNow.svelte';
    import { createSpanResizeController, type SpanResizePreview } from '../hooks/spanResize.svelte';
    import { previewColumns, type AllDayRange, type SpanEdge } from './spanResize';
    
    import { commands, useAutoQuery } from '../../../lib/safeInvoke.svelte';
    import { eventInstances, taskDates } from '../../../lib/queries';
    import { persistState, persistedKeys } from '../../../lib/persisted.svelte';
    import EventFilterButton from '../../../components/EventFilterButton.svelte';
    import { store } from '../../../lib/store.svelte';

    let {
        calendars: propCalendars,
        dragState,
        selectedEventId,
        onEventDragStart,
        onTaskDragStart,
        onAddEvent,
        onEventClick,
        onTaskClick,
        onToggleTask,
        onEventResize,
    }: {
        calendars?: AppCalendar[];
        dragState?: import('../day-timeline/types').DragState;
        selectedEventId?: string;
        onEventDragStart?: (e: PointerEvent, ev: EventInstance) => void;
        onTaskDragStart?: (e: PointerEvent, marker: TaskDateInstance) => void;
        onAddEvent?: (date: Date) => void;
        onEventClick?: (ev: EventInstance) => void;
        onTaskClick?: (marker: TaskDateInstance) => void;
        onToggleTask?: (taskId: string) => void;
        onEventResize?: (instance: EventInstance, range: AllDayRange) => void;
    } = $props();

    let view = $state<DateGridView>(DateGridView.ThreeWeeks);
    let filters = $state<FilterGroup<AppEventFilter>>(emptyFilterGroup());
    let visibility = $state<EventVisibility>({ hideFrequentRecurring: false, hideTimed: false, hideDoneTasks: false });
    let query = $state('');
    let cellsRef = $state<HTMLDivElement | null>(null);
    const now = useNow(60_000);
    let today = $derived(new Date(now.ms));

    persistState(
        persistedKeys.dateGridVisibility,
        () => visibility,
        (stored) => { visibility = stored; },
        { isValid: isEventVisibility },
    );

    persistState(
        persistedKeys.dateGridFilters,
        () => filters,
        (stored) => { filters = stored; },
        { isValid: isEventFilterGroup },
    );
    persistState(
        persistedKeys.dateGridView,
        () => view,
        (stored) => { view = stored; },
        { isValid: (stored): stored is DateGridView => Object.values(DateGridView).includes(stored as DateGridView) },
    );

    const grid = createInfiniteDateGrid(() => view);

    onMount(() => {
        if (!cellsRef) return;
        // Attach the infinite-scroll controller first so scrollend recenters
        // before week-row snapping starts its smooth alignment animation.
        const detachGrid = grid.attach(cellsRef);
        const detachSnap = attachWeekSnap(cellsRef, {
            onBeforeSnap: grid.settle,
            onSnapStart: grid.beginSnap,
        });
        return () => {
            detachSnap();
            detachGrid();
        };
    });

    const eventsQuery = useAutoQuery(commands.queryEventInstances, () =>
        eventInstances(grid.range, filters, query), { debounceMs: 150 });

    const taskDatesQuery = useAutoQuery(commands.queryTaskDates, () =>
        taskDates(grid.range), { debounceMs: 150 });

    const ownCalendarsQuery = untrack(() => propCalendars === undefined)
        ? useAutoQuery(commands.getActiveCalendars)
        : undefined;
    let allCalendars = $derived(propCalendars ?? ownCalendarsQuery?.data ?? []);

    let events = $derived((eventsQuery.data ?? []).filter(event => isEventVisible(event, visibility)));
    let calendars = $derived(calendarsForComponent(allCalendars, 'VEVENT'));
    let taskCalendars = $derived(calendarsForComponent(allCalendars, 'VTODO'));
    let taskMarkers = $derived(visibility.hideDoneTasks
        ? (taskDatesQuery.data ?? []).filter(marker => marker.status !== 'done')
        : (taskDatesQuery.data ?? []));

    let isSingleWeek = $derived(view === DateGridView.OneWeek);
    let is3Weeks = $derived(view === DateGridView.ThreeWeeks);
    let isStrip = $derived(isSingleWeek || is3Weeks);

    const MONTHS_LONG = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    const DOW_SHORT = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    function weekLabel(weekStart: Date, monthIndex: number) {
        const week = getWeekNumber(weekStart);
        const totalWeeks = isoWeeksInYear(isoWeekYear(weekStart));
        return `Week #${week.toString()}/${totalWeeks.toString()} ${MONTHS_LONG[monthIndex]}`;
    }

    let weekLayouts = $derived(
        grid.weeks.map((week) => ({
            key: ymd(week[0].date),
            week,
            layout: layoutWeekSpans(events, week.map((c) => c.date)),
        })),
    );

    let resizePreview = $state<SpanResizePreview | undefined>(undefined);

    const startSpanResize = createSpanResizeController({
        onPreview: (preview) => { resizePreview = preview; },
        onCommit: (instance, range) => { onEventResize?.(instance, range); },
    });

    function columnDateAt(weekEl: HTMLElement, clientX: number): string | undefined {
        const weekStart = weekEl.dataset.weekStart;
        if (!weekStart) return undefined;
        const rect = weekEl.getBoundingClientRect();
        if (rect.width <= 0) return undefined;
        const column = Math.min(
            DAYS_IN_WEEK - 1,
            Math.max(0, Math.floor(((clientX - rect.left) / rect.width) * DAYS_IN_WEEK)),
        );
        return ymd(addDays(parseLocalYmd(weekStart), column));
    }

    function beginSpanResize(e: PointerEvent, span: WeekSpan, edge: SpanEdge) {
        const weekEl = e.currentTarget instanceof Element
            ? e.currentTarget.closest('.cal-week')
            : null;
        if (!(weekEl instanceof HTMLElement)) return;
        startSpanResize(e, span.instance, edge, (clientX) => columnDateAt(weekEl, clientX));
    }

    function cellEvents(date: Date): EventInstance[] {
        return events.filter((e) => instanceOccursOnDay(e, date) && !isMultiDayAllDay(e));
    }

    function cellTaskDates(date: Date): TaskDateInstance[] {
        return taskMarkers.filter((marker) => taskDateOccursOnDay(marker, date));
    }

    let titleLabel = $derived(isStrip
        ? weekLabel(grid.topWeekStart, grid.dominantMonth.month)
        : `${MONTHS_LONG[grid.dominantMonth.month]} ${grid.dominantMonth.year.toString()}`);

    function shift(n: number) {
        grid.shift(n);
    }

    function handleToggleTask(taskId: string) {
        if (onToggleTask) {
            onToggleTask(taskId);
        } else {
            void store.mutate('Failed to update task', () => store.toggleTask(taskId));
        }
    }
</script>

<section class="date-grid-pane">
    {#snippet filterMenu()}
        <EventFilterButton bind:filters bind:visibility {calendars} />
    {/snippet}

    <CalendarHeader
        {titleLabel}
        {today}
        {view}
        setView={(v: DateGridView) => { view = v; }}
        setAnchor={(d: Date) => { grid.goToDate(d); }}
        {shift}
        {filterMenu}
    />

    <div class="cal-grid" class:is-strip={isStrip} class:is-grid={!isStrip}>
        <div class="cal-dow">
            {#each DOW_SHORT as d (d)}
                <div class="cal-dow-cell">{d.toLowerCase()}</div>
            {/each}
        </div>
        <div class="cal-cells-wrap">
        <div
            class="cal-cells"
            class:is-grid={!isStrip}
            class:is-strip-3={isStrip && is3Weeks}
            class:is-strip-1={isStrip && !is3Weeks}
            class:has-week-height={grid.weekHeight > 0}
            class:dragging={!!dragState}
            style={grid.weekHeight > 0 ? `--week-height: ${grid.weekHeight}px;` : undefined}
            bind:this={cellsRef}
        >
            {#each weekLayouts as { key, week, layout } (key)}
                <div class="cal-week" data-week-start={key} style="--span-lanes: {layout.laneCount}">
                    {#each week as c (ymd(c.date))}
                        <DayCell
                            cell={c}
                            {today}
                            nowMs={now.ms}
                            events={cellEvents(c.date)}
                            taskDates={cellTaskDates(c.date)}
                            {isSingleWeek}
                            {dragState}
                            {selectedEventId}
                            {onEventDragStart}
                            {onTaskDragStart}
                            {onAddEvent}
                            {onEventClick}
                            {onTaskClick}
                            onToggleTask={handleToggleTask}
                            calendars={calendars}
                            {taskCalendars}
                        />
                    {/each}
                    {#each layout.spans as span (span.occurrenceKey)}
                        {@const readOnly = isEventReadOnly(span.instance, calendars)}
                        {@const preview = resizePreview?.occurrenceKey === span.occurrenceKey ? resizePreview : undefined}
                        {@const columns = preview
                            ? previewColumns(key, preview)
                            : { startCol: span.startCol, endColExclusive: span.endColExclusive }}
                        <button
                            type="button"
                            class="cal-span"
                            class:cont-before={span.continuesBefore}
                            class:cont-after={span.continuesAfter}
                            class:is-readonly={readOnly}
                            class:is-dragging={dragState?.event?.id === span.instance.id}
                            class:is-resizing={!!preview}
                            class:is-selected={span.instance.id === selectedEventId}
                            tabindex={span.continuesBefore ? -1 : 0}
                            title={span.instance.title || 'Untitled'}
                            style="
                                --start-col: {columns.startCol};
                                --span-cols: {columns.endColExclusive - columns.startCol};
                                --lane: {span.lane};
                                cursor: {readOnly ? 'default' : 'grab'};
                                {eventColorVars(resolveEventColor(span.instance, calendars))}
                            "
                            onpointerdown={readOnly ? undefined : (e) => onEventDragStart?.(e, span.instance)}
                            onclick={(e) => {
                                e.stopPropagation();
                                onEventClick?.(span.instance);
                            }}
                        >
                            {#if !readOnly && !span.continuesBefore}
                                <span
                                    class="cal-span-handle is-start"
                                    aria-hidden="true"
                                    onpointerdown={(e) => beginSpanResize(e, span, 'start')}
                                    onclick={(e) => e.stopPropagation()}
                                ></span>
                            {/if}
                            {span.instance.title || 'Untitled'}
                            {#if !readOnly && !span.continuesAfter}
                                <span
                                    class="cal-span-handle is-end"
                                    aria-hidden="true"
                                    onpointerdown={(e) => beginSpanResize(e, span, 'end')}
                                    onclick={(e) => e.stopPropagation()}
                                ></span>
                            {/if}
                        </button>
                    {/each}
                </div>
            {/each}
        </div>
        </div>
    </div>
</section>
