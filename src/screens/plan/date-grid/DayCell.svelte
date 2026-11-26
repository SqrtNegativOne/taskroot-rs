<script lang="ts">
    import { onMount } from 'svelte';
    import type { EventInstance, AppCalendar, TaskDateInstance } from '../../../lib/domain';
    import { isEventReadOnly } from '../../../lib/domain';
    import { eventColorVars, resolveEventColor } from '../../../lib/eventColor';
    import { formatHhMm, minutesSinceMidnight, sameDay, ymd } from '../../../lib/time';
    import TaskCircle from '../../../components/TaskCircle.svelte';
    import { chipOverflow } from './overflow';

    const OPACITY_FADED = 0.4;
    const OPACITY_READ_ONLY = 0.6;
    const CHIP_GAP_PX = 2;

    type DayItem =
        | { kind: 'event'; key: string; event: EventInstance }
        | { kind: 'task'; key: string; marker: TaskDateInstance };

    let {
        cell,
        today,
        nowMs,
        events,
        taskDates = [],
        isSingleWeek,
        dragState,
        selectedEventId,
        onEventDragStart,
        onAddEvent,
        onEventClick,
        onTaskClick,
        onToggleTask,
        calendars = [],
        taskCalendars = [],
    }: {
        cell: { date: Date; outOfMonth: boolean };
        today: Date;
        nowMs: number;
        events: EventInstance[];
        taskDates?: TaskDateInstance[];
        isSingleWeek: boolean;
        dragState?: import('../day-timeline/types').DragState;
        selectedEventId?: string;
        onEventDragStart?: (e: PointerEvent, ev: EventInstance) => void;
        onAddEvent?: (date: Date) => void;
        onEventClick?: (ev: EventInstance) => void;
        onTaskClick?: (marker: TaskDateInstance) => void;
        onToggleTask?: (taskId: string) => void;
        calendars?: AppCalendar[];
        taskCalendars?: AppCalendar[];
    } = $props();

    function formatStartTime(ev: EventInstance) {
        if (ev.timing.kind !== 'timed') return 'All Day';
        return formatHhMm(minutesSinceMidnight(new Date(ev.timing.start)));
    }

    function formatTaskTime(marker: TaskDateInstance) {
        if (marker.timing.kind !== 'timed') return '';
        return formatHhMm(minutesSinceMidnight(new Date(marker.timing.start)));
    }

    function taskColor(marker: TaskDateInstance): string | undefined {
        return taskCalendars.find((calendar) => calendar.id === marker.calendarId)?.color;
    }

    let isToday = $derived(sameDay(cell.date, today));
    let isPast = $derived(cell.date < today && !isToday);
    let cellDateStr = $derived(ymd(cell.date));
    let isDragOver = $derived(dragState?.target?.kind === "grid-day" && dragState.target.date === cellDateStr);
    let canAccept = $derived(!!dragState);
    let eventsEl = $state<HTMLDivElement | undefined>(undefined);
    let probeEl = $state<HTMLElement | undefined>(undefined);
    let availableHeight = $state(0);
    let chipHeight = $state(0);

    onMount(() => {
        const el = eventsEl;
        const probe = probeEl;
        if (typeof ResizeObserver === 'undefined') return;
        const update = () => {
            availableHeight = el?.clientHeight ?? 0;
            chipHeight = probe?.getBoundingClientRect().height ?? 0;
        };
        const observer = new ResizeObserver(update);
        if (el) observer.observe(el);
        if (probe) observer.observe(probe);
        update();
        return () => observer.disconnect();
    });

    let items = $derived<DayItem[]>([
        ...events.map((event): DayItem => ({ kind: 'event', key: `event:${event.occurrenceKey}`, event })),
        ...taskDates.map((marker): DayItem => ({ kind: 'task', key: `task:${marker.occurrenceKey}`, marker })),
    ]);
    let overflow = $derived(chipOverflow(
        items.length,
        chipHeight > 0 && availableHeight > 0
            ? Math.floor((availableHeight + CHIP_GAP_PX) / (chipHeight + CHIP_GAP_PX))
            : Infinity,
    ));
    let visibleItems = $derived(items.slice(0, overflow.visible));

    function checkPastDue(ev: EventInstance): boolean {
        if (!ev.taskId || ev.timing.kind !== 'timed') return false;
        return new Date(ev.timing.end).getTime() < nowMs;
    }
</script>

<div
    data-drop-kind="grid-day"
    data-drop-date={cellDateStr}
    class="day-cell"
    class:is-out={cell.outOfMonth}
    class:is-today={isToday}
    class:is-past={isPast}
    class:is-strip={isSingleWeek}
    class:is-drag-over={isDragOver}
    class:can-accept={canAccept}
    ondblclick={(e) => {
        if (!(e.target instanceof HTMLElement)) return;
        if (e.target.closest(".day-cell-event, .day-cell-task")) return;
        if (onAddEvent) onAddEvent(cell.date);
    }}
>
    <div class="day-cell-hd">
        <span class="day-cell-num">
            {cell.date.getDate().toString().padStart(2, '0')}
        </span>
    </div>
    <div class="day-cell-events">
        <div class="day-cell-events-scroll" bind:this={eventsEl}>
            <div class="day-cell-event day-cell-chip-probe" aria-hidden="true" bind:this={probeEl}>&nbsp;</div>
            {#each visibleItems as item (item.key)}
                {#if item.kind === 'event'}
                    {@const ev = item.event}
                    {@const title = ev.title}
                    {@const isPastDue = checkPastDue(ev)}
                    {@const isAllDay = ev.timing.kind === 'allDay'}
                    {@const readOnly = isEventReadOnly(ev, calendars)}

                    <div
                        class="day-cell-event"
                        class:ev-plan={!!ev.taskId}
                        class:is-done={ev.taskStatus === 'done'}
                        class:is-readonly={readOnly}
                        class:is-selected={ev.id === selectedEventId}
                        style="
                            cursor: {readOnly ? 'default' : 'grab'};
                            opacity: {readOnly ? OPACITY_READ_ONLY : (dragState?.event?.id === ev.id ? OPACITY_FADED : 1)};
                            {eventColorVars(resolveEventColor(ev, calendars))}
                        "
                        onpointerdown={readOnly ? undefined : (e) => {
                            if (e.target instanceof Element && e.target.closest('.task-circle')) return;
                            onEventDragStart?.(e, ev);
                        }}
                        onclick={(e) => {
                            e.stopPropagation();
                            if (onEventClick) onEventClick(ev);
                        }}
                    >
                        {#if ev.taskId}
                            <TaskCircle
                                size="sm"
                                isDoneOrChecking={ev.taskStatus === 'done'}
                                onclick={(e) => {
                                    e.stopPropagation();
                                    onToggleTask?.(ev.taskId!);
                                }}
                            />
                        {/if}
                        {#if !isAllDay}
                            <span class="day-cell-event-time">
                                {formatStartTime(ev)}
                            </span>
                        {/if}
                        <span class="day-cell-event-title" style="display: flex; align-items: center; gap: 2px;">
                            {#if isPastDue}
                                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="flex-shrink: 0; color: var(--p0);">
                                    <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
                                </svg>
                            {/if}
                            <span class="day-cell-event-title-text">{title || 'Untitled'}</span>
                        </span>
                    </div>
                {:else}
                    {@const marker = item.marker}
                    {@const isDone = marker.status === 'done'}
                    {@const taskTime = formatTaskTime(marker)}
                    <div
                        class="day-cell-task"
                        class:is-done={isDone}
                        class:is-due={marker.kind === 'due'}
                        style={eventColorVars(taskColor(marker))}
                        title="{marker.kind === 'due' ? 'Due: ' : 'Start: '}{marker.title}"
                        onclick={(e) => {
                            e.stopPropagation();
                            onTaskClick?.(marker);
                        }}
                    >
                        <TaskCircle
                            size="sm"
                            isDoneOrChecking={isDone}
                            onclick={(e) => {
                                e.stopPropagation();
                                onToggleTask?.(marker.taskId);
                            }}
                        />
                        {#if taskTime}
                            <span class="day-cell-task-time">{taskTime}</span>
                        {/if}
                        <span class="day-cell-task-title">{marker.title}</span>
                    </div>
                {/if}
            {/each}
            {#if overflow.hidden > 0}
                <div class="day-cell-more">
                    +{overflow.hidden} {overflow.hidden === 1 ? 'item' : 'items'}
                </div>
            {/if}
        </div>
    </div>
    
    {#if isDragOver}
        <div class="day-cell-drop-hint">
            <span class="bracket">▸</span> {dragState?.task ? 'drop to set start' : 'drop to plan'}
        </div>
    {/if}
</div>
