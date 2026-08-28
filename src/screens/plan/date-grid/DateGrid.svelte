<script lang="ts">
    import './date-grid.css';
    import { DateGridView, DAYS_IN_WEEK } from './constants';
    import CalendarHeader from './CalendarHeader.svelte';
    import DayCell from './DayCell.svelte';
    import type { AppEvent } from '../../../lib/domain';
    import { SvelteDate } from 'svelte/reactivity';
    import { ymd } from '../../../lib/time';
    
    import { useAutoQuery } from '../../../lib/safeInvoke.svelte';
    import FilterButton from '../../../components/FilterButton.svelte';

    let {
        dragState,
        onEventDragStart,
        onAddEvent,
        onEventClick,
    }: {
        dragState?: import('../day-timeline/types').DragState;
        onEventDragStart?: (e: PointerEvent, ev: AppEvent) => void;
        onAddEvent?: (date: Date) => void;
        onEventClick?: (ev: AppEvent) => void;
    } = $props();

    let view = $state<DateGridView>(DateGridView.Month);
    let anchor = $state(new Date());
    let filters = $state<import('../../../lib/bindings/AppEventFilter.generated').AppEventFilter[]>([]);
    let query = $state('');
    let today = $state(new Date());

    let dateGridRange = $derived.by(() => {
        const d = new Date(anchor);
        let start: Date;
        let end: Date;
        if (view === DateGridView.OneWeek || view === DateGridView.Week) {
            const day = d.getDay();
            const diff = d.getDate() - day + (day === 0 ? -6 : 1);
            start = new Date(d);
            start.setDate(diff);
            start.setHours(0,0,0,0);
            end = new Date(start);
            end.setDate(start.getDate() + 7);
        } else if (view === DateGridView.ThreeWeeks) {
            const day = d.getDay();
            const diff = d.getDate() - day + (day === 0 ? -6 : 1);
            start = new Date(d);
            start.setDate(diff);
            start.setHours(0,0,0,0);
            end = new Date(start);
            end.setDate(start.getDate() + 21);
        } else {
            const first = new Date(d.getFullYear(), d.getMonth(), 1);
            const day = first.getDay();
            const diff = first.getDate() - day + (day === 0 ? -6 : 1);
            start = new Date(first);
            start.setDate(diff);
            start.setHours(0,0,0,0);
            end = new Date(start);
            end.setDate(start.getDate() + 42);
        }
        return { startDate: start.toISOString(), endDate: end.toISOString() };
    });

    const eventsQuery = useAutoQuery<AppEvent[]>('query_events', () => ({
        filters,
        query,
        startDate: dateGridRange.startDate,
        endDate: dateGridRange.endDate
    }), { debounceMs: 150 });
    
    const calendarsQuery = useAutoQuery<string[]>('get_active_calendars', () => ({}));

    let events = $derived(eventsQuery.data ?? []);
    let activeCalendars = $derived(calendarsQuery.data ?? []);

    const DAYS_IN_CALENDAR_GRID = 42;
    const DAYS_IN_THREE_WEEKS = 21;

    let isWeek = $derived(view === DateGridView.Week || view === DateGridView.OneWeek);
    let is3Weeks = $derived(view === DateGridView.ThreeWeeks);
    let isStrip = $derived(isWeek || is3Weeks);

    function startOfWeek(d: Date) {
        const nd = new SvelteDate(d);
        const day = nd.getDay();
        const diff = nd.getDate() - day + (day === 0 ? -6 : 1);
        nd.setDate(diff);
        nd.setHours(0,0,0,0);
        return nd;
    }

    function startOfMonth(d: Date) {
        return new SvelteDate(d.getFullYear(), d.getMonth(), 1);
    }

    function addDays(d: Date, days: number) {
        const nd = new SvelteDate(d);
        nd.setDate(nd.getDate() + days);
        return nd;
    }

    function getWeekNumber(d: Date) {
        const target = new SvelteDate(d.valueOf());
        const dayNr = (d.getDay() + 6) % 7;
        target.setDate(target.getDate() - dayNr + 3);
        const firstThursday = target.valueOf();
        target.setMonth(0, 1);
        if (target.getDay() !== 4) {
            target.setMonth(0, 1 + ((4 - target.getDay()) + 7) % 7);
        }
        return 1 + Math.ceil((firstThursday - target.valueOf()) / 604800000);
    }

    const MONTHS_LONG = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    const DOW_SHORT = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    function weekRangeLabel(a: Date, b: Date) {
        const prefix = `Week #${getWeekNumber(a).toString()}/52 `;
        if (a.getMonth() === b.getMonth()) {
            return `${prefix}${MONTHS_LONG[a.getMonth()]} ${a.getDate().toString()}–${b.getDate().toString()}, ${a.getFullYear().toString()}`;
        }
        return `${prefix}${MONTHS[a.getMonth()]} ${a.getDate().toString()} – ${MONTHS[b.getMonth()]} ${b.getDate().toString()}, ${b.getFullYear().toString()}`;
    }

    let cells = $derived.by(() => {
        if (view === DateGridView.OneWeek || view === DateGridView.Week) {
            const start = startOfWeek(anchor);
            return Array.from({ length: 7 }, (_, i) => ({
                date: addDays(start, i),
                outOfMonth: false,
            }));
        }
        if (view === DateGridView.ThreeWeeks) {
            const start = startOfWeek(anchor);
            return Array.from({ length: 21 }, (_, i) => ({
                date: addDays(start, i),
                outOfMonth: false,
            }));
        }
        const first = startOfMonth(anchor);
        const start = startOfWeek(first);
        const out = [];
        for (let i = 0; i < DAYS_IN_CALENDAR_GRID; i++) {
            const d = addDays(start, i);
            out.push({ date: d, outOfMonth: d.getMonth() !== anchor.getMonth() });
        }
        return out;
    });

    let displayEvents = $derived(events); // TODO: apply filters and sort

    let titleLabel = $derived(isStrip
        ? weekRangeLabel(cells[0].date, cells[cells.length - 1].date)
        : `${MONTHS_LONG[anchor.getMonth()]} ${anchor.getFullYear().toString()}`);

    function shift(n: number) {
        const d = new SvelteDate(anchor);
        if (isWeek) d.setDate(d.getDate() + DAYS_IN_WEEK * n);
        else if (is3Weeks) d.setDate(d.getDate() + DAYS_IN_THREE_WEEKS * n);
        else d.setMonth(d.getMonth() + n);
        anchor = d;
    }
</script>

<section class="date-grid-pane">
    {#snippet filterMenu()}
        <FilterButton
            bind:filters
            columns={[{ id: 'calendar', label: 'Calendar' }]}
            getValuesForColumn={(col: string) => col === 'calendar' ? activeCalendars : []}
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
        <div class="cal-cells" class:is-grid={!isStrip} class:is-strip-3={isStrip && is3Weeks} class:is-strip-1={isStrip && !is3Weeks}>
            {#each cells as c (ymd(c.date))}
                {@const cellDateStr = ymd(c.date)}
                {@const cellStart = new Date(`${cellDateStr}T00:00:00`).getTime()}
                {@const cellEnd = cellStart + 86400000}
                
                <DayCell
                    cell={c}
                    {today}
                    events={displayEvents.filter(e => {
                        const sStr = e.startTime.includes('T') || e.startTime.includes(' ') ? e.startTime.replace(' ', 'T') : e.startTime + 'T00:00:00';
                        const eStr = e.endTime.includes('T') || e.endTime.includes(' ') ? e.endTime.replace(' ', 'T') : e.endTime + 'T00:00:00';
                        const eStart = new Date(sStr).getTime();
                        const eEnd = new Date(eStr).getTime();
                        return eStart < cellEnd && eEnd > cellStart;
                    })}
                    {isWeek}
                    {dragState}
                    {onEventDragStart}
                    {onAddEvent}
                    {onEventClick}
                />
            {/each}
        </div>
    </div>
</section>
