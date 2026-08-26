<script lang="ts">
    import './day-timeline.css';
    import { onMount } from 'svelte';
    import type { DragState, LaidEvent, PlanDayLayout } from './types';
    import { PX_PER_MIN } from './constants';
    import type { AppEvent } from '../../../lib/domain';
    import { useTauriQuery } from '../../../lib/safeInvoke.svelte';
    import { addDays, minutesSinceMidnight, sameDay, ymd } from '../../../lib/time';

    import TimelineHeader from './components/TimelineHeader.svelte';
    import DayColumn from './components/DayColumn.svelte';

    let {
        events,
        filterMenu,
        eventFilters = [],
        eventQuery = '',
        today,
        timelineDate,
        setTimelineDate,
        dragState,
        setDragState,
        onResizeEvent,
        onMoveEvent,
        onEventClick,
        onAddEvent,
    }: {
        events: AppEvent[];
        filterMenu?: import('svelte').Snippet;
        eventFilters?: import('../../../lib/bindings/AppEventFilter.generated').AppEventFilter[];
        eventQuery?: string;
        today: Date;
        timelineDate: Date;
        setTimelineDate: (d: Date) => void;
        dragState?: DragState;
        setDragState?: (ds: DragState | undefined) => void;
        onResizeEvent?: (id: string, startTime: string, endTime: string) => void;
        onMoveEvent?: (id: string, startTime: string, endTime: string) => void;
        onEventClick?: (ev: AppEvent) => void;
        onAddEvent?: (d: Date, start: number, end: number) => void;
    } = $props();

    let viewDate = $derived(timelineDate);
    let isToday = $derived(sameDay(viewDate, today));
    
    let numDays = $state(1);
    
    let dates = $derived(Array.from({ length: numDays }, (_, i) => addDays(viewDate, i)));
    
    let layoutQuery = useTauriQuery<PlanDayLayout[]>('query_plan_layout');

    let planLayout = $derived.by(() => {
        const layoutMap: Record<string, LaidEvent[]> = {};
        for (const day of layoutQuery.data ?? []) {
            layoutMap[day.date] = day.events;
        }
        return layoutMap;
    });

    $effect(() => {
        void events; // Trigger re-run when events change
        void layoutQuery.execute({ dates: dates.map(ymd), filters: eventFilters, query: eventQuery });
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
    <TimelineHeader
        {viewDate}
        {isToday}
        {today}
        {setTimelineDate}
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
