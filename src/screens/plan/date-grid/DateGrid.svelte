<script lang="ts">
    import './date-grid.css';
    import { DateGridView, DAYS_IN_WEEK } from './constants';
    import CalendarHeader from './CalendarHeader.svelte';
    import DayCell from './DayCell.svelte';
    import type { EventInstance, AppCalendar, AppEventFilter, TaskDateInstance } from '../../../lib/domain';
    import { calendarsForComponent, instanceOccursOnDay, isEventReadOnly, taskDateOccursOnDay } from '../../../lib/domain';
    import { eventColorVars, resolveEventColor } from '../../../lib/eventColor';
    import { isMultiDayAllDay, layoutWeekSpans } from './spanLayout';
    import { getWeekNumber, isoWeeksInYear, isoWeekYear, ymd } from '../../../lib/time';
    import { buildDateGrid, shiftDateGridAnchor, type GridCell } from './dates';
    import { isEventVisible, isEventVisibility, type EventVisibility } from '../../../lib/eventVisibility';
    import { useNow } from '../../../lib/useNow.svelte';
    
    import { useAutoQuery } from '../../../lib/safeInvoke.svelte';
    import { activeCalendars, eventInstances, taskDates } from '../../../lib/queries';
    import { persistState, persistedKeys } from '../../../lib/persisted.svelte';
    import EventFilterButton from '../../../components/EventFilterButton.svelte';

    let {
        dragState,
        selectedEventId,
        onEventDragStart,
        onAddEvent,
        onEventClick,
        onTaskClick,
    }: {
        dragState?: import('../day-timeline/types').DragState;
        selectedEventId?: string;
        onEventDragStart?: (e: PointerEvent, ev: EventInstance) => void;
        onAddEvent?: (date: Date) => void;
        onEventClick?: (ev: EventInstance) => void;
        onTaskClick?: (marker: TaskDateInstance) => void;
    } = $props();

    let view = $state<DateGridView>(DateGridView.ThreeWeeks);
    let anchor = $state(new Date());
    let filters = $state<AppEventFilter[]>([]);
    let visibility = $state<EventVisibility>({ hideFrequentRecurring: false, hideTimed: false });
    let query = $state('');
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
        { isValid: Array.isArray },
    );
    persistState(
        persistedKeys.dateGridView,
        () => view,
        (stored) => { view = stored; },
        { isValid: (stored): stored is DateGridView => Object.values(DateGridView).includes(stored as DateGridView) },
    );

    let grid = $derived(buildDateGrid(anchor, view));
    let cells = $derived(grid.cells);

    const eventsQuery = useAutoQuery<EventInstance[]>('query_event_instances', () =>
        eventInstances(grid.range, filters, query), { debounceMs: 150 });

    const taskDatesQuery = useAutoQuery<TaskDateInstance[]>('query_task_dates', () =>
        taskDates(grid.range), { debounceMs: 150 });

    const calendarsQuery = useAutoQuery<AppCalendar[]>('get_active_calendars', activeCalendars);

    let events = $derived((eventsQuery.data ?? []).filter(event => isEventVisible(event, visibility)));
    let calendars = $derived(calendarsForComponent(calendarsQuery.data ?? [], 'VEVENT'));
    let taskCalendars = $derived(calendarsForComponent(calendarsQuery.data ?? [], 'VTODO'));
    let taskMarkers = $derived(taskDatesQuery.data ?? []);

    let isSingleWeek = $derived(view === DateGridView.OneWeek);
    let is3Weeks = $derived(view === DateGridView.ThreeWeeks);
    let isStrip = $derived(isSingleWeek || is3Weeks);

    const MONTHS_LONG = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    const DOW_SHORT = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    function weekLabel(a: Date) {
        const week = getWeekNumber(a);
        const totalWeeks = isoWeeksInYear(isoWeekYear(a));
        return `Week #${week.toString()}/${totalWeeks.toString()} ${MONTHS_LONG[anchor.getMonth()]}`;
    }

    let weeks = $derived.by(() => {
        const rows: GridCell[][] = [];
        for (let i = 0; i < cells.length; i += DAYS_IN_WEEK) rows.push(cells.slice(i, i + DAYS_IN_WEEK));
        return rows;
    });

    let weekLayouts = $derived(
        weeks.map((week) => ({
            key: ymd(week[0].date),
            week,
            layout: layoutWeekSpans(events, week.map((c) => c.date)),
        })),
    );

    function cellEvents(date: Date): EventInstance[] {
        return events.filter((e) => instanceOccursOnDay(e, date) && !isMultiDayAllDay(e));
    }

    function cellTaskDates(date: Date): TaskDateInstance[] {
        return taskMarkers.filter((marker) => taskDateOccursOnDay(marker, date));
    }

    let titleLabel = $derived(isStrip
        ? weekLabel(cells[0].date)
        : `${MONTHS_LONG[anchor.getMonth()]} ${anchor.getFullYear().toString()}`);

    function shift(n: number) {
        anchor = shiftDateGridAnchor(anchor, view, n);
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
        setAnchor={(d: Date) => { anchor = d; }}
        {shift}
        {filterMenu}
    />

    <div class="cal-grid" class:is-strip={isStrip} class:is-grid={!isStrip}>
        <div class="cal-dow">
            {#each DOW_SHORT as d (d)}
                <div class="cal-dow-cell">{d.toLowerCase()}</div>
            {/each}
        </div>
        <div class="cal-cells" class:is-grid={!isStrip} class:is-strip-3={isStrip && is3Weeks} class:is-strip-1={isStrip && !is3Weeks} class:dragging={!!dragState}>
            {#each weekLayouts as { key, week, layout } (key)}
                <div class="cal-week" style="--span-lanes: {layout.laneCount}">
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
                            {onAddEvent}
                            {onEventClick}
                            {onTaskClick}
                            calendars={calendars}
                            {taskCalendars}
                        />
                    {/each}
                    {#each layout.spans as span (span.occurrenceKey)}
                        {@const readOnly = isEventReadOnly(span.instance, calendars)}
                        <button
                            type="button"
                            class="cal-span"
                            class:cont-before={span.continuesBefore}
                            class:cont-after={span.continuesAfter}
                            class:is-readonly={readOnly}
                            class:is-dragging={dragState?.event?.id === span.instance.id}
                            class:is-selected={span.instance.id === selectedEventId}
                            tabindex={span.continuesBefore ? -1 : 0}
                            title={span.instance.title || 'Untitled'}
                            style="
                                --start-col: {span.startCol};
                                --span-cols: {span.endColExclusive - span.startCol};
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
                            {span.instance.title || 'Untitled'}
                        </button>
                    {/each}
                </div>
            {/each}
        </div>
    </div>
</section>
