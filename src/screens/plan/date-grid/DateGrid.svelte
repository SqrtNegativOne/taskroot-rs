<script lang="ts">
    import './date-grid.css';
    import { DateGridView, DAYS_IN_WEEK } from './constants';
    import CalendarHeader from './CalendarHeader.svelte';
    import DayCell from './DayCell.svelte';
    import type { EventInstance, AppCalendar } from '../../../lib/domain';
    import { instanceOccursOnDay, isEventReadOnly } from '../../../lib/domain';
    import { eventColorVars } from '../../../lib/eventColor';
    import { isMultiDayAllDay, layoutWeekSpans } from './spanLayout';
    import { addDays, getWeekNumber, isoWeeksInYear, isoWeekYear, startOfMonth, startOfWeek, ymd } from '../../../lib/time';
    import { useNow } from '../../../lib/useNow.svelte';
    
    import { useAutoQuery } from '../../../lib/safeInvoke.svelte';
    import { activeCalendars, eventInstances } from '../../../lib/queries';
    import { persistState, persistedKeys } from '../../../lib/persisted.svelte';
    import FilterButton from '../../../components/FilterButton.svelte';

    let {
        dragState,
        onEventDragStart,
        onAddEvent,
        onEventClick,
    }: {
        dragState?: import('../day-timeline/types').DragState;
        onEventDragStart?: (e: PointerEvent, ev: EventInstance) => void;
        onAddEvent?: (date: Date) => void;
        onEventClick?: (ev: EventInstance) => void;
    } = $props();

    let view = $state<DateGridView>(DateGridView.ThreeWeeks);
    let anchor = $state(new Date());
    let filters = $state<import('../../../lib/bindings/AppEventFilter.generated').AppEventFilter[]>([]);
    let query = $state('');
    const now = useNow(60_000);
    let today = $derived(new Date(now.ms));

    const DAYS_IN_CALENDAR_GRID = 42;
    const DAYS_IN_THREE_WEEKS = 21;

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

    let dateGridRange = $derived.by(() => {
        const start = view === DateGridView.Month
            ? startOfWeek(startOfMonth(anchor))
            : startOfWeek(anchor);
        const dayCount = view === DateGridView.OneWeek
            ? DAYS_IN_WEEK
            : view === DateGridView.ThreeWeeks
                ? DAYS_IN_THREE_WEEKS
                : DAYS_IN_CALENDAR_GRID;
        const end = addDays(start, dayCount);
        return { startDate: ymd(start), endDate: ymd(end) };
    });

    const eventsQuery = useAutoQuery<EventInstance[]>('query_event_instances', () =>
        eventInstances(dateGridRange, filters, query), { debounceMs: 150 });

    const calendarsQuery = useAutoQuery<AppCalendar[]>('get_active_calendars', activeCalendars);

    let events = $derived(eventsQuery.data ?? []);
    let calendars = $derived(calendarsQuery.data ?? []);

    let isSingleWeek = $derived(view === DateGridView.OneWeek);
    let is3Weeks = $derived(view === DateGridView.ThreeWeeks);
    let isStrip = $derived(isSingleWeek || is3Weeks);

    const MONTHS_LONG = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    const DOW_SHORT = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    function weekRangeLabel(a: Date, b: Date) {
        const week = getWeekNumber(a);
        const totalWeeks = isoWeeksInYear(isoWeekYear(a));
        const prefix = `Week #${week.toString()}/${totalWeeks.toString()} `;
        if (a.getMonth() === b.getMonth()) {
            return `${prefix}${MONTHS_LONG[a.getMonth()]} ${a.getDate().toString()}–${b.getDate().toString()}, ${a.getFullYear().toString()}`;
        }
        return `${prefix}${MONTHS[a.getMonth()]} ${a.getDate().toString()} – ${MONTHS[b.getMonth()]} ${b.getDate().toString()}, ${b.getFullYear().toString()}`;
    }

    let cells = $derived.by(() => {
        if (view === DateGridView.OneWeek || view === DateGridView.ThreeWeeks) {
            const start = startOfWeek(anchor);
            const length = view === DateGridView.OneWeek ? DAYS_IN_WEEK : DAYS_IN_THREE_WEEKS;
            return Array.from({ length }, (_, i) => ({
                date: addDays(start, i),
                outOfMonth: false,
            }));
        }
        const start = startOfWeek(startOfMonth(anchor));
        const out = [];
        for (let i = 0; i < DAYS_IN_CALENDAR_GRID; i++) {
            const d = addDays(start, i);
            out.push({ date: d, outOfMonth: d.getMonth() !== anchor.getMonth() });
        }
        return out;
    });

    type GridCell = { date: Date; outOfMonth: boolean };

    let weeks = $derived.by(() => {
        const rows: GridCell[][] = [];
        for (let i = 0; i < cells.length; i += 7) rows.push(cells.slice(i, i + 7));
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

    let titleLabel = $derived(isStrip
        ? weekRangeLabel(cells[0].date, cells[cells.length - 1].date)
        : `${MONTHS_LONG[anchor.getMonth()]} ${anchor.getFullYear().toString()}`);

    function shift(n: number) {
        if (isSingleWeek) {
            anchor = addDays(anchor, DAYS_IN_WEEK * n);
            return;
        }
        if (is3Weeks) {
            anchor = addDays(anchor, DAYS_IN_THREE_WEEKS * n);
            return;
        }
        const d = new Date(anchor);
        d.setMonth(d.getMonth() + n);
        anchor = d;
    }
</script>

<section class="date-grid-pane">
    {#snippet filterMenu()}
        <FilterButton
            bind:filters
            columns={[{ id: 'remotecollectionid', label: 'Calendar' }]}
            getValuesForColumn={(col: string) => col === 'remotecollectionid' ? calendars.map(c => ({ label: c.summary || c.id, value: c.id })) : []}
            align="right"
        />
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
                            {isSingleWeek}
                            {dragState}
                            {onEventDragStart}
                            {onAddEvent}
                            {onEventClick}
                            calendars={calendars}
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
                            tabindex={span.continuesBefore ? -1 : 0}
                            title={span.instance.title}
                            style="
                                --start-col: {span.startCol};
                                --span-cols: {span.endColExclusive - span.startCol};
                                --lane: {span.lane};
                                cursor: {readOnly ? 'default' : 'grab'};
                                {eventColorVars(span.instance.color)}
                            "
                            onpointerdown={readOnly ? undefined : (e) => onEventDragStart?.(e, span.instance)}
                            onclick={(e) => {
                                e.stopPropagation();
                                onEventClick?.(span.instance);
                            }}
                        >
                            {span.instance.title}
                        </button>
                    {/each}
                </div>
            {/each}
        </div>
    </div>
</section>
