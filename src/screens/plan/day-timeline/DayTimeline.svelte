<script lang="ts">
    import './day-timeline.css';
    import { onMount, untrack } from 'svelte';
    import type { DragState } from './types';
    import { PX_PER_MIN, NUM_DAYS_OPTIONS } from './constants';
    import type { EventInstance, AppCalendar } from '../../../lib/domain';
    import { calendarsForComponent, isEventReadOnly } from '../../../lib/domain';
    import { addDays, minutesSinceMidnight, sameDay, ymd } from '../../../lib/time';
    import { bucketEventsByDay } from './bucketing';
    import TimelineHeader from './components/TimelineHeader.svelte';
    import DayColumn from './components/DayColumn.svelte';
    import RecurringActionModal from '../../../components/RecurringActionModal.svelte';
    import { useAutoQuery } from '../../../lib/safeInvoke.svelte';
    import { activeCalendars, eventInstances } from '../../../lib/queries';
    import { persistState, persistedKeys } from '../../../lib/persisted.svelte';
    import FilterButton from '../../../components/FilterButton.svelte';
    import { createReschedulePrompt } from '../hooks/reschedule.svelte';
    import { attachDaySnap } from './scrollSnap';

    let {
        dragState,
        onEventClick,
        onAddEvent,
        variant = 'plan',
    }: {
        dragState?: DragState;
        onEventClick?: (ev: EventInstance) => void;
        onAddEvent?: (d: Date, start: number, end: number) => Promise<string | undefined>;
        variant?: 'plan' | 'sidebar';
    } = $props();

    function isNumDaysValue(value: unknown): value is number {
        return typeof value === 'number' && NUM_DAYS_OPTIONS.includes(value);
    }

    const reschedulePrompt = createReschedulePrompt();

    function onResizeEvent(instance: EventInstance, startTime: string, endTime: string) {
        requestReschedule(instance, startTime, endTime, 'Failed to resize event');
    }

    function onMoveEvent(instance: EventInstance, startTime: string, endTime: string) {
        requestReschedule(instance, startTime, endTime, 'Failed to move event');
    }

    function requestReschedule(
        instance: EventInstance,
        startTime: string,
        endTime: string,
        operation: string,
    ) {
        if (isEventReadOnlyById(instance.id)) return;
        reschedulePrompt.request(
            instance,
            startTime,
            endTime,
            instance.timing.kind === 'allDay',
            operation,
        );
    }

    function isEventReadOnlyById(id: string): boolean {
        return (eventsQuery.data ?? []).some(
            (ev) => ev.id === id && isEventReadOnly(ev, calendars),
        );
    }

    let eventFilters = $state<import('../../../lib/bindings/AppEventFilter.generated').AppEventFilter[]>([]);
    let eventQuery = $state('');
    let timelineDate = $state(new Date());
    let today = $state(new Date());
    let numDays = $state(1);

    const timelineKeys = untrack(() =>
        variant === 'sidebar'
            ? { filters: persistedKeys.sidebarTimelineFilters, numDays: persistedKeys.sidebarTimelineNumDays }
            : { filters: persistedKeys.dayTimelineFilters, numDays: persistedKeys.dayTimelineNumDays },
    );

    persistState(
        timelineKeys.filters,
        () => eventFilters,
        (stored) => { eventFilters = stored; },
        { isValid: Array.isArray },
    );
    persistState(
        timelineKeys.numDays,
        () => numDays,
        (stored) => { numDays = stored; },
        { isValid: isNumDaysValue },
    );

    let viewDate = $derived(timelineDate);
    let isToday = $derived(sameDay(viewDate, today));
    
    let dates = $derived(Array.from({ length: numDays }, (_, i) => addDays(viewDate, i)));
    
    import { layoutEvents } from './layout';

    let dateRange = $derived.by(() => {
        if (dates.length === 0) return { startDate: '', endDate: '' };
        return { startDate: ymd(dates[0]), endDate: ymd(dates[dates.length - 1]) };
    });

    const eventsQuery = useAutoQuery<EventInstance[]>('query_event_instances', () =>
        eventInstances(dateRange, eventFilters, eventQuery), { debounceMs: 150 });

    const calendarsQuery = useAutoQuery<AppCalendar[]>('get_active_calendars', activeCalendars);

    let calendars = $derived(calendarsForComponent(calendarsQuery.data ?? [], 'VEVENT'));

    let planLayout = $derived.by(() => {
        const layoutMap: Record<string, import('./types').LaidEvent[]> = {};
        for (const date of dates) {
            layoutMap[ymd(date)] = [];
        }

        const bucketed = bucketEventsByDay(eventsQuery.data ?? [], dates);

        for (const [dateStr, bucketEvents] of Object.entries(bucketed)) {
            layoutMap[dateStr] = layoutEvents(bucketEvents);
        }
        
        return layoutMap;
    });
    
    // Auto scroll logic
    let scrollRef = $state<HTMLDivElement | null>(null);
    const AUTO_SCROLL_LEAD_MINUTES = 120;
    onMount(() => {
        if (!scrollRef) return;
        const scrollMins = Math.max(0, minutesSinceMidnight(new Date()) - AUTO_SCROLL_LEAD_MINUTES);
        scrollRef.scrollTop = scrollMins * PX_PER_MIN;
        return attachDaySnap(scrollRef);
    });

    const dayHeaderFormat = new Intl.DateTimeFormat('en-US', {
        weekday: 'short',
        month: 'short',
        day: 'numeric',
    });
</script>

<section class="day-pane">
    {#snippet filterMenu()}
        <FilterButton
            bind:filters={eventFilters}
            columns={[{ id: 'remotecollectionid', label: 'Calendar' }]}
            getValuesForColumn={(col: string) => col === 'remotecollectionid' ? calendars.map((c) => ({ label: c.summary || c.id, value: c.id })) : []}
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
        <div class="day-cols">
            <div class="day-head">
                {#each dates as d (ymd(d))}
                    <div class="day-head-cell" class:is-today={sameDay(d, today)} data-day={ymd(d)}>
                        {dayHeaderFormat.format(d)}
                    </div>
                {/each}
            </div>
            <div class="day-row">
                {#each dates as d, i (ymd(d))}
                    <DayColumn
                        date={d}
                        {today}
                        laid={planLayout[ymd(d)] ?? []}
                        {dragState}
                        {onResizeEvent}
                        {onMoveEvent}
                        {onEventClick}
                        {onAddEvent}
                        calendars={calendars}
                        showTimeLabels={i === 0}
                    />
                {/each}
            </div>
        </div>
    </div>

    <RecurringActionModal
        isOpen={reschedulePrompt.isOpen}
        actionType="move"
        modes={['this', 'all']}
        onConfirm={reschedulePrompt.confirm}
        onCancel={reschedulePrompt.cancel}
    />
</section>
