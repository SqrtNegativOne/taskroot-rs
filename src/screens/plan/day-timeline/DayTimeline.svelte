<script lang="ts">
    import './day-timeline.css';
    import { onMount, untrack } from 'svelte';
    import type { DragState } from './types';
    import { PX_PER_MIN, NUM_DAYS_OPTIONS } from './constants';
    import type { EventInstance, AppCalendar, EventScope } from '../../../lib/domain';
    import { isEventReadOnly } from '../../../lib/domain';
    import { addDays, minutesSinceMidnight, sameDay, ymd } from '../../../lib/time';
    import { bucketEventsByDay } from './bucketing';
    import TimelineHeader from './components/TimelineHeader.svelte';
    import DayColumn from './components/DayColumn.svelte';
    import RecurringActionModal from '../../../components/RecurringActionModal.svelte';
    import { useAutoQuery } from '../../../lib/safeInvoke.svelte';
    import { persistState, persistedKeys } from '../../../lib/persisted.svelte';
    import FilterButton from '../../../components/FilterButton.svelte';
    import { store } from '../../../lib/store.svelte';

    let {
        dragState,
        onEventClick,
        onAddEvent,
        variant = 'plan',
    }: {
        dragState?: DragState;
        onEventClick?: (ev: EventInstance) => void;
        onAddEvent?: (d: Date, start: number, end: number) => void;
        variant?: 'plan' | 'sidebar';
    } = $props();

    function isNumDaysValue(value: unknown): value is number {
        return typeof value === 'number' && NUM_DAYS_OPTIONS.includes(value);
    }

    type PendingReschedule = {
        instance: EventInstance;
        startTime: string;
        endTime: string;
        isAllDay: boolean;
        operation: string;
    };

    let pendingReschedule = $state<PendingReschedule | undefined>(undefined);
    let moveModalOpen = $state(false);

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
        const isAllDay = instance.timing.kind === 'allDay';
        if (!instance.recurring) {
            commitReschedule(instance, startTime, endTime, isAllDay, 'all', operation);
            return;
        }
        pendingReschedule = { instance, startTime, endTime, isAllDay, operation };
        moveModalOpen = true;
    }

    function commitReschedule(
        instance: EventInstance,
        startTime: string,
        endTime: string,
        isAllDay: boolean,
        scope: EventScope,
        operation: string,
    ) {
        void store.mutate(operation, () =>
            store.rescheduleEvent(instance.id, startTime, endTime, isAllDay, scope, instance.occurrenceStart),
        );
    }

    function handleMoveScope(scope: EventScope) {
        moveModalOpen = false;
        const pending = pendingReschedule;
        pendingReschedule = undefined;
        if (!pending) return;
        commitReschedule(
            pending.instance,
            pending.startTime,
            pending.endTime,
            pending.isAllDay,
            scope,
            pending.operation,
        );
    }

    function isEventReadOnlyById(id: string): boolean {
        return (eventsQuery.data ?? []).some(
            (ev) => ev.id === id && isEventReadOnly(ev, activeCalendars),
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
        if (dates.length === 0) return { start: '', end: '' };
        return { start: ymd(dates[0]), end: ymd(dates[dates.length - 1]) };
    });

    const eventsQuery = useAutoQuery<EventInstance[]>('query_event_instances', () => ({
        filters: eventFilters,
        query: eventQuery,
        startDate: dateRange.start,
        endDate: dateRange.end
    }), { debounceMs: 150 });
    
    const calendarsQuery = useAutoQuery<AppCalendar[]>('get_active_calendars', () => ({}));

    let activeCalendars = $derived(calendarsQuery.data ?? []);

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
            columns={[{ id: 'remotecollectionid', label: 'Calendar' }]}
            getValuesForColumn={(col: string) => col === 'remotecollectionid' ? activeCalendars.map((c) => ({ label: c.summary || c.id, value: c.id })) : []}
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
                    {onResizeEvent}
                    {onMoveEvent}
                    {onEventClick}
                    {onAddEvent}
                    calendars={activeCalendars}
                    showTimeLabels={i === 0}
                />
            {/each}
        </div>
    </div>

    <RecurringActionModal
        isOpen={moveModalOpen}
        actionType="move"
        modes={['this', 'all']}
        onConfirm={handleMoveScope}
        onCancel={() => { moveModalOpen = false; pendingReschedule = undefined; }}
    />
</section>
