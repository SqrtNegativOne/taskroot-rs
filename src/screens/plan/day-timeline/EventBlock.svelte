<script lang="ts">
    import {
        COMPACT_EVENT_HEIGHT_PX,
        DRAG_THRESHOLD_PX,
        HOURS_PER_DAY,
        MINUTES_IN_HOUR,
        PIXELS_PER_HOUR,
        PX_PER_MIN,
        SNAP_MIN,
    } from './constants';
    import type { AppEvent } from '../../../lib/domain';
    import { createPointerGestureRecognizer } from './hooks/pointerGesture.svelte';

    const MIN_EVENT_HEIGHT_PX = 18;
    const SHORT_EVENT_DURATION_MINS = 30;
    const DEFAULT_LABEL_OFFSET_PX = PIXELS_PER_HOUR;

    let {
        event,
        startMins,
        endMins,
        lane,
        lanes,
        onResize,
        onMove,
        onEventClick,
        labelOffset = DEFAULT_LABEL_OFFSET_PX,
    }: {
        event: AppEvent;
        startMins: number;
        endMins: number;
        lane: number;
        lanes: number;
        onResize?: (id: string, start: number, end: number) => void;
        onMove?: (id: string, start: number, end: number) => void;
        onEventClick?: (e: AppEvent) => void;
        labelOffset?: number;
    } = $props();

    let dragOffset = $state<number | undefined>(undefined);

    const title = $derived(event.title);
    // Since task data is minimal in AppEvent in the new domain, we can mock or extend it later.
    const isRecurring = $derived(!!event.rrule || !!event.recurringEventId);

    const trackPointerGesture = createPointerGestureRecognizer();

    function clamp(value: number, min: number, max: number) {
        return Math.min(max, Math.max(min, value));
    }

    function snappedDeltaMinutes(deltaPx: number) {
        return Math.round(deltaPx / PX_PER_MIN / SNAP_MIN) * SNAP_MIN;
    }

    function onResizeStart(edge: 'top' | 'bottom') {
        return (e: PointerEvent) => {
            e.stopPropagation();
            e.preventDefault();
            const startY = e.clientY;
            const startStart = startMins;
            const startEnd = endMins;

            trackPointerGesture({
                onMove: (ev) => {
                    const dm = snappedDeltaMinutes(ev.clientY - startY);
                    if (edge === 'bottom') {
                        const newEnd = clamp(startEnd + dm, startStart + SNAP_MIN, HOURS_PER_DAY * MINUTES_IN_HOUR);
                        onResize?.(event.id, startStart, newEnd);
                    } else {
                        const newStart = clamp(startStart + dm, 0, startEnd - SNAP_MIN);
                        onResize?.(event.id, newStart, startEnd);
                    }
                },
            });
        };
    }

    function onBodyDown(e: PointerEvent) {
        if (!(e.target instanceof Element)) return;
        if (e.target.closest('.day-event-handle')) return;
        if (e.button !== 0) return;

        e.preventDefault();
        const startY = e.clientY;
        const startStart = startMins;
        const startEnd = endMins;
        let moved = false;
        let finalDm = 0;

        trackPointerGesture({
            onMove: (ev) => {
                const dy = ev.clientY - startY;
                if (!moved && Math.abs(dy) < DRAG_THRESHOLD_PX) return;
                moved = true;

                finalDm = clamp(
                    snappedDeltaMinutes(dy),
                    -startStart,
                    HOURS_PER_DAY * MINUTES_IN_HOUR - startEnd,
                );
                dragOffset = finalDm;
            },
            onEnd: () => {
                if (!moved) {
                    onEventClick?.(event);
                    return;
                }
                dragOffset = undefined;
                if (finalDm !== 0) {
                    onMove?.(
                        event.id,
                        startStart + finalDm,
                        startStart + finalDm + (startEnd - startStart),
                    );
                }
            },
            onCancel: () => {
                dragOffset = undefined;
            },
        });
    }

    function pad2(n: number) {
        return n.toString().padStart(2, '0');
    }
</script>

{#snippet renderBlock(start: number, end: number, isGhost: boolean, isFloating: boolean)}
    {@const top = start * PX_PER_MIN}
    {@const height = (end - start) * PX_PER_MIN}
    {@const compact = height < COMPACT_EVENT_HEIGHT_PX}
    {@const isShort = (end - start) <= SHORT_EVENT_DURATION_MINS}
    
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
        class="day-event"
        class:ev-plan={!!event.taskId}
        class:is-compact={compact}
        class:is-short={isShort && !compact}
        class:is-ghost={isGhost}
        class:is-floating={isFloating}
        style="
            top: {top}px;
            height: {Math.max(height, MIN_EVENT_HEIGHT_PX)}px;
            left: calc({labelOffset}px + ((100% - {labelOffset}px) / {lanes}) * {lane});
            width: calc(((100% - {labelOffset}px) / {lanes}) - 2px);
            {event.color ? `--ev-color: ${event.color}; --ev-bg: ${event.bgColor ?? event.color + '40'};` : ''}
        "
        onpointerdown={isGhost ? undefined : onBodyDown}
    >
        <div
            role="separator"
            tabindex="-1"
            class="day-event-handle day-event-handle-top"
            onpointerdown={isGhost ? undefined : onResizeStart('top')}
        ></div>
        
        <div class="day-event-inner">
            <div class="day-event-title" style="display: flex; align-items: center; gap: 4px;">
                {#if isRecurring}
                    <span style="flex-shrink: 0; font-size: 14px;">↻</span>
                {/if}
                {title}
            </div>
            <div class="day-event-time">
                {pad2(Math.floor(Math.round(start) / MINUTES_IN_HOUR))}:{pad2(Math.round(start) % MINUTES_IN_HOUR)} – 
                {pad2(Math.floor(Math.round(end) / MINUTES_IN_HOUR))}:{pad2(Math.round(end) % MINUTES_IN_HOUR)}
            </div>
        </div>
        
        <div
            role="separator"
            tabindex="-1"
            class="day-event-handle day-event-handle-bottom"
            onpointerdown={isGhost ? undefined : onResizeStart('bottom')}
        >
            <span class="day-event-handle-grip">═</span>
        </div>
    </div>
{/snippet}

{#if dragOffset !== undefined}
    {@render renderBlock(startMins, endMins, true, false)}
    {@render renderBlock(startMins + dragOffset, endMins + dragOffset, false, true)}
{:else}
    {@render renderBlock(startMins, endMins, false, false)}
{/if}
