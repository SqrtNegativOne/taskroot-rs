<script lang="ts">
    import './day-timeline.css';
    import { onMount } from 'svelte';
    import type { DragState } from './types';
    import { PX_PER_MIN } from './constants';
    import type { AppEvent } from '../../../lib/domain';
    import { addDays, minutesSinceMidnight, sameDay, ymd } from '../../../lib/time';
    import TimelineHeader from './components/TimelineHeader.svelte';
    import DayColumn from './components/DayColumn.svelte';
    import { useAutoQuery } from '../../../lib/safeInvoke.svelte';
    import FilterButton from '../../../components/FilterButton.svelte';
    import { store } from '../../../lib/store.svelte';

    let {
        dragState,
        setDragState,
        onEventClick,
        onAddEvent,
    }: {
        dragState?: DragState;
        setDragState?: (ds: DragState | undefined) => void;
        onEventClick?: (ev: AppEvent) => void;
        onAddEvent?: (d: Date, start: number, end: number) => void;
    } = $props();

    function onResizeEvent(id: string, startTime: string, endTime: string) {
        // Find the event inside planLayout
        let ev: AppEvent | undefined;
        for (const dayEvents of Object.values(planLayout)) {
            const found = dayEvents.find(e => e.event.id === id);
            if (found) { ev = found.event; break; }
        }
        if (ev) void store.updateEvent({ ...ev, startTime, endTime });
    }

    function onMoveEvent(id: string, startTime: string, endTime: string) {
        let ev: AppEvent | undefined;
        for (const dayEvents of Object.values(planLayout)) {
            const found = dayEvents.find(e => e.event.id === id);
            if (found) { ev = found.event; break; }
        }
        if (ev) void store.updateEvent({ ...ev, startTime, endTime });
    }

    let eventFilters = $state<import('../../../lib/bindings/AppEventFilter.generated').AppEventFilter[]>([]);
    let eventQuery = $state('');
    let timelineDate = $state(new Date());
    let today = $state(new Date());

    let viewDate = $derived(timelineDate);
    let isToday = $derived(sameDay(viewDate, today));
    
    let numDays = $state(1);
    
    let dates = $derived(Array.from({ length: numDays }, (_, i) => addDays(viewDate, i)));
    
    import { layoutEvents, type DayLayoutEvent } from './layout';

    let dateRange = $derived.by(() => {
        if (dates.length === 0) return { start: '', end: '' };
        return { start: ymd(dates[0]), end: ymd(dates[dates.length - 1]) };
    });

    const eventsQuery = useAutoQuery<AppEvent[]>('query_events', () => ({
        filters: eventFilters,
        query: eventQuery,
        startDate: dateRange.start,
        endDate: dateRange.end
    }), { debounceMs: 150 });
    
    const calendarsQuery = useAutoQuery<string[]>('get_active_calendars', () => ({}));

    let activeCalendars = $derived(calendarsQuery.data ?? []);

    let planLayout = $derived.by(() => {
        const layoutMap: Record<string, import('./types').LaidEvent[]> = {};
        for (const date of dates) {
            layoutMap[ymd(date)] = [];
        }
        
        const events = eventsQuery.data ?? [];
        const bucketed: Record<string, DayLayoutEvent[]> = {};
        
        for (const ev of events) {
            const startStr = ev.startTime;
            const endStr = ev.endTime;
            // Simple string prefix matching for days
            for (const date of dates) {
                const dateStr = ymd(date);
                if (startStr.startsWith(dateStr) || endStr.startsWith(dateStr) || (startStr < dateStr && endStr > dateStr)) {
                    // Extract HH:MM
                    const s = new Date(startStr);
                    const e = new Date(endStr);
                    const startMins = s.getHours() * 60 + s.getMinutes();
                    const endMins = e.getHours() * 60 + e.getMinutes();
                    
                    if (!bucketed[dateStr]) bucketed[dateStr] = [];
                    bucketed[dateStr].push({ event: ev, startMins, endMins });
                }
            }
        }

        for (const [dateStr, bucketEvents] of Object.entries(bucketed)) {
            layoutMap[dateStr] = layoutEvents(bucketEvents);
        }
        
        return layoutMap;
    });
    
    // Auto scroll logic
    let scrollRef = $state<HTMLDivElement | null>(null);
    const AUTO_SCROLL_LEAD_MINUTES = 120;
    onMount(() => {
        if (scrollRef) {
            const scrollMins = Math.max(0, minutesSinceMidnight(new Date()) - AUTO_SCROLL_LEAD_MINUTES);
            scrollRef.scrollTop = scrollMins * PX_PER_MIN;
        }
    });
</script>

<section class="day-pane">
    {#snippet filterMenu()}
        <FilterButton
            bind:filters={eventFilters}
            columns={[{ id: 'calendar', label: 'Calendar' }]}
            getValuesForColumn={(col: string) => col === 'calendar' ? activeCalendars : []}
            align="right"
        />
    {/snippet}

    <TimelineHeader
        {viewDate}
        {isToday}
        {today}
        setTimelineDate={(d: Date) => { timelineDate = d; }}
        {filterMenu}
        {numDays}
        setNumDays={(n: number) => { numDays = n; }}
    />

    <div class="day-scroll" bind:this={scrollRef}>
        <div style="display: flex; flex-direction: row; width: 100%;">
            {#each dates as d, i (ymd(d))}
                <DayColumn
                    date={d}
                    {today}
                    laid={planLayout[ymd(d)] ?? []}
                    {dragState}
                    {setDragState}
                    {onResizeEvent}
                    {onMoveEvent}
                    {onEventClick}
                    {onAddEvent}
                    showTimeLabels={i === 0}
                />
            {/each}
        </div>
    </div>
</section>
