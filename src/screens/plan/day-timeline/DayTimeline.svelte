<script lang="ts">
    import './day-timeline.css';
    import { onMount } from 'svelte';
    import type { DragState, LaidEvent, PlanDayLayout } from './types';
    import type { AppEvent } from '../../../lib/domain';
    import { SvelteDate } from 'svelte/reactivity';
    import { invoke } from '@tauri-apps/api/core';

    import TimelineHeader from './components/TimelineHeader.svelte';
    import DayColumn from './components/DayColumn.svelte';

    let {
        events,
        filterMenu,
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

    function ymd(d: Date) {
        return d.toISOString().split('T')[0];
    }
    
    function addDays(d: Date, days: number) {
        const nd = new SvelteDate(d);
        nd.setDate(nd.getDate() + days);
        return nd;
    }

    let viewDate = $derived(timelineDate);
    let isToday = $derived(ymd(viewDate) === ymd(today));
    
    let numDays = $state(1);
    
    let dates = $derived(Array.from({ length: numDays }, (_, i) => addDays(viewDate, i)));
    
    let planLayout = $state<Record<string, LaidEvent[]>>({});
    
    $effect(() => {
        const ds = dates.map(ymd);
        void events; // Trigger re-run when events change
        
        invoke<PlanDayLayout[]>('get_plan_layout', { dates: ds }).then((res) => {
            const layoutMap: Record<string, LaidEvent[]> = {};
            for (const day of res) {
                layoutMap[day.date] = day.events;
            }
            planLayout = layoutMap;
        }).catch(console.error);
    });
    
    // Auto scroll logic
    let scrollRef = $state<HTMLDivElement | null>(null);
    onMount(() => {
        if (scrollRef) {
            const now = new Date();
            const pxPerMin = 56 / 60;
            const scrollMins = Math.max(0, now.getHours() * 60 + now.getMinutes() - 120);
            scrollRef.scrollTop = scrollMins * pxPerMin;
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
