<script lang="ts">
    import './day-timeline.css';
    import { onMount, untrack } from 'svelte';
    import type { DragState } from './types';
    import { MINUTES_IN_HOUR, PX_PER_MIN, NUM_DAYS_OPTIONS } from './constants';
    import { resolveStartHour } from '../snapping';
    import type { AppCalendar, EventInstance, AppEventFilter, FilterGroup } from '../../../lib/domain';
    import { calendarsForComponent, emptyFilterGroup, isEventFilterGroup, isEventReadOnly } from '../../../lib/domain';
    import { minutesSinceMidnight, sameDay, ymd } from '../../../lib/time';
    import { bucketEventsByDay } from './bucketing';
    import { layoutEvents } from './layout';
    import TimelineControls from './components/TimelineControls.svelte';
    import DayColumn from './components/DayColumn.svelte';
    import HourGutter from './components/HourGutter.svelte';
    import RecurringActionModal from '../../../components/RecurringActionModal.svelte';
    import { commands, useAutoQuery } from '../../../lib/safeInvoke.svelte';
    import { eventInstances } from '../../../lib/queries';
    import { persistState, persistedKeys } from '../../../lib/persisted.svelte';
    import { useNow } from '../../../lib/useNow.svelte';
    import EventFilterButton from '../../../components/EventFilterButton.svelte';
    import { createReschedulePrompt } from '../hooks/reschedule.svelte';
    import { attachDaySnap } from './scrollSnap';
    import { createInfiniteTimeline } from './infiniteTimeline.svelte';
    import { store } from '../../../lib/store.svelte';

    let {
        calendars: propCalendars,
        dragState,
        selectedEventId,
        onEventClick,
        onAddEvent,
        variant = 'plan',
        startHour: propStartHour,
    }: {
        calendars?: AppCalendar[];
        dragState?: DragState;
        selectedEventId?: string;
        onEventClick?: (ev: EventInstance) => void;
        onAddEvent?: (d: Date, start: number, end: number) => Promise<string | undefined>;
        variant?: 'plan' | 'sidebar';
        startHour?: number;
    } = $props();

    let startHour = $derived(resolveStartHour(propStartHour, store.settings?.earliest_wake_up_time));
    let startMinute = $derived(startHour * MINUTES_IN_HOUR);

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

    function handleToggleTask(taskId: string) {
        void store.mutate('Failed to update task', () => store.toggleTask(taskId));
    }

    let eventFilters = $state<FilterGroup<AppEventFilter>>(emptyFilterGroup());
    let eventQuery = $state('');
    let numDays = $state(1);
    let filterOpen = $state(false);
    const now = useNow(60_000);
    let today = $derived(new Date(now.ms));

    const timelineKeys = untrack(() =>
        variant === 'sidebar'
            ? { filters: persistedKeys.sidebarTimelineFilters, numDays: persistedKeys.sidebarTimelineNumDays }
            : { filters: persistedKeys.dayTimelineFilters, numDays: persistedKeys.dayTimelineNumDays },
    );

    persistState(
        timelineKeys.filters,
        () => eventFilters,
        (stored) => { eventFilters = stored; },
        { isValid: isEventFilterGroup },
    );
    persistState(
        timelineKeys.numDays,
        () => numDays,
        (stored) => { numDays = stored; },
        { isValid: isNumDaysValue },
    );

    const timeline = createInfiniteTimeline(() => numDays);

    const eventsQuery = useAutoQuery(commands.queryEventInstances, () =>
        eventInstances(timeline.dateRange, eventFilters, eventQuery), { debounceMs: 150 });

    const ownCalendarsQuery = untrack(() => propCalendars === undefined)
        ? useAutoQuery(commands.getActiveCalendars)
        : undefined;

    let calendars = $derived(calendarsForComponent(propCalendars ?? ownCalendarsQuery?.data ?? [], 'VEVENT'));

    let planLayout = $derived.by(() => {
        const layoutMap: Record<string, import('./types').LaidEvent[]> = {};
        for (const date of timeline.dates) {
            layoutMap[ymd(date)] = [];
        }

        const bucketed = bucketEventsByDay(eventsQuery.data ?? [], timeline.dates);

        for (const [dateStr, bucketEvents] of Object.entries(bucketed)) {
            layoutMap[dateStr] = layoutEvents(
                bucketEvents.filter((ev) => ev.endMins > startMinute),
            );
        }

        return layoutMap;
    });

    let scrollRef = $state<HTMLDivElement | null>(null);
    const AUTO_SCROLL_LEAD_MINUTES = 120;
    onMount(() => {
        if (!scrollRef) return;
        const scrollMins = Math.max(
            0,
            minutesSinceMidnight(new Date()) - AUTO_SCROLL_LEAD_MINUTES - startMinute,
        );
        scrollRef.scrollTop = scrollMins * PX_PER_MIN;
        // Attach the infinite-scroll controller first so scrollend recenters
        // before day snapping starts its smooth alignment animation.
        const detachTimeline = timeline.attach(scrollRef);
        const detachSnap = attachDaySnap(scrollRef, {
            onBeforeSnap: timeline.settle,
            onSnapStart: timeline.beginSnap,
        });
        return () => {
            detachSnap();
            detachTimeline();
        };
    });

    const weekdayFormat = new Intl.DateTimeFormat('en-US', { weekday: 'short' });
    const monthFormat = new Intl.DateTimeFormat('en-US', { month: 'short' });
    const dayOfMonthFormat = new Intl.DateTimeFormat('en-US', { day: 'numeric' });

    function formatDayHeader(d: Date): string {
        const weekday = weekdayFormat.format(d).toLowerCase();
        const day = dayOfMonthFormat.format(d);
        if (d.getDate() === 1) {
            return `${weekday}, ${monthFormat.format(d).toLowerCase()} ${day}`;
        }
        return `${weekday}, ${day}`;
    }
</script>

<section class="day-pane">
    {#snippet filterMenu()}
        <EventFilterButton
            bind:filters={eventFilters}
            {calendars}
            align="left"
            onOpenChange={(isOpen) => { filterOpen = isOpen; }}
        />
    {/snippet}

    <div class="day-scroll" bind:this={scrollRef}>
        <div
            class="day-cols"
            style="--day-count: {timeline.dates.length};{timeline.colWidth > 0 ? ` --day-col-w: ${timeline.colWidth}px;` : ''}"
        >
            <div class="day-head">
                <div class="day-gutter">
                    <TimelineControls
                        viewDate={timeline.viewDate}
                        {today}
                        setTimelineDate={(d: Date) => { timeline.goToDate(d); }}
                        {filterMenu}
                        {numDays}
                        setNumDays={(n: number) => { numDays = n; }}
                        keepOpen={filterOpen}
                    />
                </div>
                {#each timeline.dates as d (ymd(d))}
                    <div
                        class="day-head-cell"
                        class:is-today={sameDay(d, today)}
                        data-day={ymd(d)}
                    >
                        <span class="day-head-label">{formatDayHeader(d)}</span>
                    </div>
                {/each}
            </div>
            <div class="day-row">
                <div class="day-gutter day-gutter-hours"><HourGutter {startHour} /></div>
                {#each timeline.dates as d (ymd(d))}
                    <DayColumn
                        date={d}
                        {today}
                        laid={planLayout[ymd(d)] ?? []}
                        {startHour}
                        {dragState}
                        {selectedEventId}
                        {onResizeEvent}
                        {onMoveEvent}
                        {onEventClick}
                        {onAddEvent}
                        onToggleTask={handleToggleTask}
                        calendars={calendars}
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
