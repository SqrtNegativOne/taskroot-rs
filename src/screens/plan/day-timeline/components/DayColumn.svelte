<script lang="ts">
    import { HOURS_PER_DAY, MINUTES_IN_HOUR, PX_PER_MIN, SNAP_MIN, type DragState, type LaidEvent } from '../types';
    import type { AppEvent } from '../../../../lib/domain';

    import EventBlock from '../EventBlock.svelte';
    import TimeGridBackground from './TimeGridBackground.svelte';
    import CurrentTimeLine from './CurrentTimeLine.svelte';
    import CreationPreview from './CreationPreview.svelte';
    import DropPreview from './DropPreview.svelte';

    let {
        date,
        today,
        laid,
        dragState,
        setDragState,
        onResizeEvent,
        onMoveEvent,
        onEventClick,
        onAddEvent,
        showTimeLabels = true,
    }: {
        date: Date;
        today: Date;
        laid: LaidEvent[];
        dragState?: DragState;
        setDragState?: (ds: DragState | undefined) => void;
        onResizeEvent?: (id: string, startTime: string, endTime: string) => void;
        onMoveEvent?: (id: string, startTime: string, endTime: string) => void;
        onEventClick?: (ev: AppEvent) => void;
        onAddEvent?: (d: Date, start: number, end: number) => void;
        showTimeLabels?: boolean;
    } = $props();

    function ymd(d: Date) {
        return d.toISOString().split('T')[0];
    }
    
    function sameDay(a: Date, b: Date) {
        return ymd(a) === ymd(b);
    }

    let isToday = $derived(sameDay(date, today));
    let cellDateStr = $derived(ymd(date));
    
    // Event handlers
    function handleResize(id: string, newStartMins: number, newEndMins: number) {
        const cellStart = new Date(`${cellDateStr}T00:00:00`).getTime();
        const newStartDt = new Date(cellStart + newStartMins * 60000);
        const newEndDt = new Date(cellStart + newEndMins * 60000);
        if (onResizeEvent) onResizeEvent(id, newStartDt.toISOString(), newEndDt.toISOString());
    }

    function handleMove(id: string, newStartMins: number, newEndMins: number) {
        const cellStart = new Date(`${cellDateStr}T00:00:00`).getTime();
        const newStartDt = new Date(cellStart + newStartMins * 60000);
        const newEndDt = new Date(cellStart + newEndMins * 60000);
        if (onMoveEvent) onMoveEvent(id, newStartDt.toISOString(), newEndDt.toISOString());
    }

    // Grid creation logic
    let containerRef = $state<HTMLDivElement | null>(null);
    let createPreview = $state<{ start: number; end: number } | undefined>(undefined);

    function onGridPointerDown(e: PointerEvent) {
        if (!(e.target instanceof Element)) return;
        if (e.target.closest('.day-event') || e.target.closest('.day-now')) return;
        if (e.button !== 0) return;
        e.preventDefault();

        if (!containerRef) return;
        const rect = containerRef.getBoundingClientRect();
        const startY = e.clientY - rect.top;
        const startMin = Math.round(startY / PX_PER_MIN / SNAP_MIN) * SNAP_MIN;

        let active = false;

        const move = (ev: PointerEvent) => {
            active = true;
            const currentY = ev.clientY - rect.top;
            const moveMin = Math.round(currentY / PX_PER_MIN / SNAP_MIN) * SNAP_MIN;
            const s = Math.min(startMin, moveMin);
            const eMin = Math.max(startMin, moveMin);
            createPreview = {
                start: s,
                end: eMin === s ? s + SNAP_MIN : eMin,
            };
        };

        const up = (ev: PointerEvent) => {
            window.removeEventListener('pointermove', move);
            window.removeEventListener('pointerup', up);
            if (!active) {
                if (onAddEvent) onAddEvent(date, startMin, startMin + MINUTES_IN_HOUR);
                return;
            }
            const currentY = ev.clientY - rect.top;
            const moveMin = Math.round(currentY / PX_PER_MIN / SNAP_MIN) * SNAP_MIN;
            const s = Math.min(startMin, moveMin);
            const eMin = Math.max(startMin, moveMin);
            const finalEnd = eMin === s ? s + SNAP_MIN : eMin;
            createPreview = undefined;
            if (onAddEvent) onAddEvent(date, s, finalEnd);
        };

        window.addEventListener('pointermove', move);
        window.addEventListener('pointerup', up);
    }

    let dropPreview = $derived(
        dragState?.target?.kind === 'day-time' && dragState.target.date === cellDateStr
            ? dragState.target
            : undefined
    );
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    class="day-grid"
    bind:this={containerRef}
    style="
        position: relative;
        flex: 1;
        min-width: 200px;
        height: {HOURS_PER_DAY * MINUTES_IN_HOUR * PX_PER_MIN}px;
        border-right: 1px solid var(--border-soft);
    "
    data-drop-kind="day-time"
    data-drop-date={cellDateStr}
    onpointerdown={onGridPointerDown}
    onpointerenter={() => {
        if (dragState?.event && setDragState) {
            setDragState({
                ...dragState,
                target: {
                    kind: 'day-time',
                    date: cellDateStr,
                    start: dragState.target?.start ?? 0,
                    end: dragState.target?.end ?? 0
                }
            });
        }
    }}
>
    <div style="position: absolute; top: -32px; left: 0; right: 0; text-align: center; font-weight: bold; font-size: 0.9em; color: {isToday ? 'var(--accent)' : 'var(--fg)'}; padding-bottom: 8px;">
        {Intl.DateTimeFormat('en-US', { weekday: 'short', month: 'short', day: 'numeric' }).format(date)}
    </div>

    <TimeGridBackground {isToday} showLabels={showTimeLabels} />
    
    {#if isToday}
        <CurrentTimeLine showLabels={showTimeLabels} />
    {/if}

    {#each laid as { event, startMins, endMins, lane, lanes } (event.id)}
        <EventBlock
            {event}
            {startMins}
            {endMins}
            {lane}
            {lanes}
            onResize={handleResize}
            onMove={handleMove}
            {onEventClick}
            labelOffset={showTimeLabels ? 56 : 8}
        />
    {/each}

    {#if createPreview}
        <CreationPreview preview={createPreview} />
    {/if}

    {#if dropPreview?.minute !== undefined}
        <DropPreview target={dropPreview} />
    {/if}
</div>
