<script lang="ts">
    import { MINUTES_IN_HOUR, PX_PER_MIN, SNAP_MIN } from '../constants';
    import type { CreationRange, DragState, LaidEvent } from '../types';
    import type { EventInstance, AppCalendar } from '../../../../lib/domain';
    import { isEventReadOnly } from '../../../../lib/domain';
    import { MS_PER_MINUTE, parseLocalYmd, sameDay, ymd } from '../../../../lib/time';
    import { MINUTES_PER_DAY, snapPixels } from '../../snapping';

    import EventBlock from '../EventBlock.svelte';
    import TimeGridBackground from './TimeGridBackground.svelte';
    import CurrentTimeLine from './CurrentTimeLine.svelte';
    import CreationPreview from './CreationPreview.svelte';
    import DropPreview from './DropPreview.svelte';
    import { createPointerGestureRecognizer } from '../hooks/pointerGesture.svelte';

    let {
        date,
        today,
        laid,
        startHour = 0,
        dragState,
        selectedEventId,
        onResizeEvent,
        onMoveEvent,
        onEventClick,
        onAddEvent,
        onToggleTask,
        calendars = [],
    }: {
        date: Date;
        today: Date;
        laid: LaidEvent[];
        startHour?: number;
        dragState?: DragState;
        selectedEventId?: string;
        onResizeEvent?: (event: EventInstance, startTime: string, endTime: string) => void;
        onMoveEvent?: (event: EventInstance, startTime: string, endTime: string) => void;
        onEventClick?: (ev: EventInstance) => void;
        onAddEvent?: (d: Date, start: number, end: number) => Promise<string | undefined>;
        onToggleTask?: (taskId: string) => void;
        calendars?: AppCalendar[];
    } = $props();

    let startMinute = $derived(startHour * MINUTES_IN_HOUR);
    let visibleLaid = $derived(laid.filter((item) => item.endMins > startMinute));
    let isToday = $derived(sameDay(date, today));
    let cellDateStr = $derived(ymd(date));
    let readOnlyEventIds = $derived(
        new Set(laid.filter((l) => isEventReadOnly(l.event, calendars)).map((l) => l.event.id)),
    );
    
    function emitTimeRange(
        event: EventInstance,
        newStartMins: number,
        newEndMins: number,
        handler?: (event: EventInstance, startTime: string, endTime: string) => void,
    ) {
        if (!handler || readOnlyEventIds.has(event.id)) return;
        const cellStart = parseLocalYmd(cellDateStr).getTime();
        const newStartDt = new Date(cellStart + newStartMins * MS_PER_MINUTE);
        const newEndDt = new Date(cellStart + newEndMins * MS_PER_MINUTE);
        handler(event, newStartDt.toISOString(), newEndDt.toISOString());
    }

    // Grid creation logic
    let containerRef = $state<HTMLDivElement | null>(null);
    let createPreview = $state<CreationRange | undefined>(undefined);
    // Layout of `laid` when the created event was submitted; the ghost stays up
    // until the post-mutation query refresh replaces it with the real block.
    // `$state.raw` keeps the exact reference so identity comparison works.
    let createBaseline = $state.raw<LaidEvent[] | undefined>(undefined);

    const trackPointerGesture = createPointerGestureRecognizer();

    function previewRange(startMin: number, currentMin: number): CreationRange {
        const s = Math.min(startMin, currentMin);
        const eMin = Math.max(startMin, currentMin);
        return { start: s, end: eMin === s ? s + SNAP_MIN : eMin };
    }

    function defaultCreateRange(startMin: number): CreationRange {
        return { start: startMin, end: startMin + MINUTES_IN_HOUR };
    }

    async function finishCreate(range: CreationRange, baseline: LaidEvent[]) {
        const id = await onAddEvent?.(date, range.start, range.end);
        if (id !== undefined && !laid.some((l) => l.event.id === id)) {
            createBaseline = baseline;
            return;
        }
        createPreview = undefined;
    }

    function pointerMinute(clientY: number, gridTop: number): number {
        return Math.min(
            MINUTES_PER_DAY,
            Math.max(startMinute, startMinute + snapPixels(clientY - gridTop)),
        );
    }

    function onGridPointerDown(e: PointerEvent) {
        if (!(e.target instanceof Element)) return;
        if (e.target.closest('.day-event') || e.target.closest('.day-now')) return;
        if (e.button !== 0) return;
        e.preventDefault();

        if (!containerRef) return;
        const rect = containerRef.getBoundingClientRect();
        const startMin = pointerMinute(e.clientY, rect.top);
        let active = false;

        createBaseline = undefined;
        createPreview = defaultCreateRange(startMin);

        trackPointerGesture({
            onMove: (ev) => {
                active = true;
                const moveMin = pointerMinute(ev.clientY, rect.top);
                createPreview = previewRange(startMin, moveMin);
            },
            onEnd: (ev) => {
                const baseline = laid;
                const range = active
                    ? previewRange(startMin, pointerMinute(ev.clientY, rect.top))
                    : defaultCreateRange(startMin);
                createPreview = range;
                void finishCreate(range, baseline);
            },
            onCancel: () => {
                createBaseline = undefined;
                createPreview = undefined;
            },
        });
    }

    $effect(() => {
        const baseline = createBaseline;
        if (baseline === undefined || laid === baseline) return;
        createBaseline = undefined;
        createPreview = undefined;
    });

    let dropPreview = $derived(
        dragState?.target?.kind === 'day-time' && dragState.target.date === cellDateStr
            ? dragState.target
            : undefined
    );
</script>

<div
    class="day-grid"
    bind:this={containerRef}
    style="
        position: relative;
        height: {(MINUTES_PER_DAY - startMinute) * PX_PER_MIN}px;
        border-right: 1px solid var(--border-soft);
    "
    data-drop-kind="day-time"
    data-drop-date={cellDateStr}
    data-start-minute={startMinute}
    onpointerdown={onGridPointerDown}
>
    <TimeGridBackground {startHour} />
    
    {#if isToday}
        <CurrentTimeLine {startMinute} />
    {/if}

    {#each visibleLaid as { event, startMins, endMins, lane, lanes } (event.occurrenceKey)}
        <EventBlock
            {event}
            {startMins}
            {endMins}
            {lane}
            {lanes}
            {startMinute}
            selected={event.id === selectedEventId}
            readOnly={readOnlyEventIds.has(event.id)}
            {calendars}
            onResize={(ev, s, e) => emitTimeRange(ev, s, e, onResizeEvent)}
            onMove={(ev, s, e) => emitTimeRange(ev, s, e, onMoveEvent)}
            {onEventClick}
            {onToggleTask}
        />
    {/each}

    {#if createPreview}
        <CreationPreview preview={createPreview} {startMinute} />
    {/if}

    {#if dropPreview?.minute !== undefined}
        <DropPreview target={dropPreview} {startMinute} />
    {/if}
</div>
