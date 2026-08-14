<script lang="ts">
    import { HOURS_PER_DAY, MINUTES_IN_HOUR, PX_PER_MIN, SNAP_MIN } from './types';
    import { DRAG_THRESHOLD_PX, COMPACT_EVENT_HEIGHT_PX } from './constants';
    import type { AppEvent } from '../../../lib/domain';

    const MIN_EVENT_HEIGHT_PX = 18;
    const SHORT_EVENT_DURATION_MINS = 30;
    const DEFAULT_LABEL_OFFSET_PX = 56;

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

    function onResizeStart(edge: 'top' | 'bottom') {
        return (e: PointerEvent) => {
            e.stopPropagation();
            e.preventDefault();
            const startY = e.clientY;
            const startStart = startMins;
            const startEnd = endMins;

            const move = (ev: PointerEvent) => {
                const dy = ev.clientY - startY;
                const dm = Math.round(dy / PX_PER_MIN / SNAP_MIN) * SNAP_MIN;
                
                if (edge === 'bottom') {
                    const newEnd = Math.max(
                        startStart + SNAP_MIN,
                        Math.min(HOURS_PER_DAY * MINUTES_IN_HOUR, startEnd + dm)
                    );
                    if (onResize) onResize(event.id, startStart, newEnd);
                } else {
                    const newStart = Math.max(
                        0,
                        Math.min(startEnd - SNAP_MIN, startStart + dm)
                    );
                    if (onResize) onResize(event.id, newStart, startEnd);
                }
            };

            const up = () => {
                window.removeEventListener('pointermove', move);
                window.removeEventListener('pointerup', up);
            };

            window.addEventListener('pointermove', move);
            window.addEventListener('pointerup', up);
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

        const move = (ev: PointerEvent) => {
            const dy = ev.clientY - startY;
            if (!moved && Math.abs(dy) < DRAG_THRESHOLD_PX) return;
            moved = true;
            
            const dm = Math.round(dy / PX_PER_MIN / SNAP_MIN) * SNAP_MIN;
            const minDm = -startStart;
            const maxDm = HOURS_PER_DAY * MINUTES_IN_HOUR - startEnd;
            finalDm = Math.max(minDm, Math.min(maxDm, dm));
            dragOffset = finalDm;
        };

        const up = () => {
            window.removeEventListener('pointermove', move);
            window.removeEventListener('pointerup', up);
            
            if (!moved) {
                if (onEventClick) onEventClick(event);
                return;
            }
            dragOffset = undefined;
            if (finalDm !== 0 && onMove) {
                onMove(
                    event.id,
                    startStart + finalDm,
                    startStart + finalDm + (startEnd - startStart)
                );
            }
        };

        window.addEventListener('pointermove', move);
        window.addEventListener('pointerup', up);
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
        class="day-event ev-{event.type}"
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

<!-- eslint-disable @typescript-eslint/no-confusing-void-expression -->
{#if dragOffset !== undefined}
    {@render renderBlock(startMins, endMins, true, false)}
    {@render renderBlock(startMins + dragOffset, endMins + dragOffset, false, true)}
{:else}
    {@render renderBlock(startMins, endMins, false, false)}
{/if}
