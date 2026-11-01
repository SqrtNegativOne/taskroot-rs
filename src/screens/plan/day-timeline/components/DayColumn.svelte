<script lang="ts">
    import { MINUTES_IN_HOUR, PIXELS_PER_HOUR, PX_PER_MIN, SNAP_MIN } from '../constants';
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
        dragState,
        onResizeEvent,
        onMoveEvent,
        onEventClick,
        onAddEvent,
        calendars = [],
        showTimeLabels = true,
    }: {
        date: Date;
        today: Date;
        laid: LaidEvent[];
        dragState?: DragState;
        onResizeEvent?: (event: EventInstance, startTime: string, endTime: string) => void;
        onMoveEvent?: (event: EventInstance, startTime: string, endTime: string) => void;
        onEventClick?: (ev: EventInstance) => void;
        onAddEvent?: (d: Date, start: number, end: number) => Promise<string | undefined>;
        calendars?: AppCalendar[];
        showTimeLabels?: boolean;
    } = $props();

    let isToday = $derived(sameDay(date, today));
    let cellDateStr = $derived(ymd(date));
    let readOnlyEventIds = $derived(
        new Set(laid.filter((l) => isEventReadOnly(l.event, calendars)).map((l) => l.event.id)),
    );
    
    // Event handlers
    function handleResize(event: EventInstance, newStartMins: number, newEndMins: number) {
        if (readOnlyEventIds.has(event.id)) return;
        const cellStart = parseLocalYmd(cellDateStr).getTime();
        const newStartDt = new Date(cellStart + newStartMins * MS_PER_MINUTE);
        const newEndDt = new Date(cellStart + newEndMins * MS_PER_MINUTE);
        if (onResizeEvent) onResizeEvent(event, newStartDt.toISOString(), newEndDt.toISOString());
    }

    function handleMove(event: EventInstance, newStartMins: number, newEndMins: number) {
        if (readOnlyEventIds.has(event.id)) return;
        const cellStart = parseLocalYmd(cellDateStr).getTime();
        const newStartDt = new Date(cellStart + newStartMins * MS_PER_MINUTE);
        const newEndDt = new Date(cellStart + newEndMins * MS_PER_MINUTE);
        if (onMoveEvent) onMoveEvent(event, newStartDt.toISOString(), newEndDt.toISOString());
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

    function onGridPointerDown(e: PointerEvent) {
        if (!(e.target instanceof Element)) return;
        if (e.target.closest('.day-event') || e.target.closest('.day-now')) return;
        if (e.button !== 0) return;
        e.preventDefault();

        if (!containerRef) return;
        const rect = containerRef.getBoundingClientRect();
        const startMin = snapPixels(e.clientY - rect.top);
        let active = false;

        createBaseline = undefined;
        createPreview = defaultCreateRange(startMin);

        trackPointerGesture({
            onMove: (ev) => {
                active = true;
                const moveMin = snapPixels(ev.clientY - rect.top);
                createPreview = previewRange(startMin, moveMin);
            },
            onEnd: (ev) => {
                const baseline = laid;
                const range = active
                    ? previewRange(startMin, snapPixels(ev.clientY - rect.top))
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
        --day-label-offset: {showTimeLabels ? PIXELS_PER_HOUR : 0}px;
        position: relative;
        flex: 1;
        min-width: 200px;
        height: {MINUTES_PER_DAY * PX_PER_MIN}px;
        border-right: 1px solid var(--border-soft);
    "
    data-drop-kind="day-time"
    data-drop-date={cellDateStr}
    onpointerdown={onGridPointerDown}
>
    <div style="position: absolute; top: -32px; left: 0; right: 0; text-align: center; font-weight: bold; font-size: 0.9em; color: {isToday ? 'var(--accent)' : 'var(--fg)'}; padding-bottom: 8px;">
        {Intl.DateTimeFormat('en-US', { weekday: 'short', month: 'short', day: 'numeric' }).format(date)}
    </div>

    <TimeGridBackground {isToday} showLabels={showTimeLabels} />
    
    {#if isToday}
        <CurrentTimeLine showLabels={showTimeLabels} />
    {/if}

    {#each laid as { event, startMins, endMins, lane, lanes } (event.occurrenceKey)}
        <EventBlock
            {event}
            {startMins}
            {endMins}
            {lane}
            {lanes}
            readOnly={readOnlyEventIds.has(event.id)}
            onResize={handleResize}
            onMove={handleMove}
            {onEventClick}
        />
    {/each}

    {#if createPreview}
        <CreationPreview preview={createPreview} />
    {/if}

    {#if dropPreview?.minute !== undefined}
        <DropPreview target={dropPreview} />
    {/if}
</div>
