<script lang="ts">
    import {
        COMPACT_EVENT_HEIGHT_PX,
        DRAG_THRESHOLD_PX,
        EVENT_INSET_PX,
        PX_PER_MIN,
        SNAP_MIN,
    } from './constants';
    import type { AppCalendar, EventInstance } from '../../../lib/domain';
    import { resolveEventColor } from '../../../lib/eventColor';
    import { eventColorVars } from '../../../lib/themes/eventTheme.svelte';
    import { formatHhMm } from '../../../lib/time';
    import { MINUTES_PER_DAY, snapPixels } from '../snapping';
    import { createPointerGestureRecognizer } from './hooks/pointerGesture.svelte';
    import TaskCircle from '../../../components/TaskCircle.svelte';

    const MIN_EVENT_HEIGHT_PX = 18;
    const SHORT_EVENT_DURATION_MINS = 30;

    let {
        event,
        startMins,
        endMins,
        lane,
        lanes,
        startMinute = 0,
        selected,
        onResize,
        onMove,
        onEventClick,
        onToggleTask,
        readOnly = false,
        calendars = [],
    }: {
        event: EventInstance;
        startMins: number;
        endMins: number;
        lane: number;
        lanes: number;
        startMinute?: number;
        selected?: boolean;
        onResize?: (event: EventInstance, start: number, end: number) => void;
        onMove?: (event: EventInstance, start: number, end: number) => void;
        onEventClick?: (e: EventInstance) => void;
        onToggleTask?: (taskId: string) => void;
        readOnly?: boolean;
        calendars?: AppCalendar[];
    } = $props();

    let dragOffset = $state<number | undefined>(undefined);

    const title = $derived(event.title);
    const isRecurring = $derived(event.recurring);

    const trackPointerGesture = createPointerGestureRecognizer();

    function clamp(value: number, min: number, max: number) {
        return Math.min(max, Math.max(min, value));
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
                    const dm = snapPixels(ev.clientY - startY);
                    if (edge === 'bottom') {
                        const newEnd = clamp(startEnd + dm, startStart + SNAP_MIN, MINUTES_PER_DAY);
                        onResize?.(event, startStart, newEnd);
                    } else {
                        const minStart = Math.min(startStart, startMinute);
                        const newStart = clamp(startStart + dm, minStart, startEnd - SNAP_MIN);
                        onResize?.(event, newStart, startEnd);
                    }
                },
            });
        };
    }

    function onBodyDown(e: PointerEvent) {
        if (!(e.target instanceof Element)) return;
        if (e.target.closest('.day-event-handle, .task-circle')) return;
        if (e.button !== 0) return;

        if (readOnly) {
            onEventClick?.(event);
            return;
        }

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
                    snapPixels(dy),
                    Math.min(0, startMinute - startStart),
                    MINUTES_PER_DAY - startEnd,
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
                        event,
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

</script>

{#snippet renderBlock(start: number, end: number, isGhost: boolean, isFloating: boolean)}
    {@const visibleStart = Math.max(start, startMinute)}
    {@const visibleDuration = Math.max(0, end - visibleStart)}
    {@const slotHeight = visibleDuration * PX_PER_MIN}
    {@const top = (visibleStart - startMinute) * PX_PER_MIN + EVENT_INSET_PX}
    {@const height = Math.max(slotHeight - 2 * EVENT_INSET_PX, MIN_EVENT_HEIGHT_PX)}
    {@const compact = slotHeight < COMPACT_EVENT_HEIGHT_PX}
    {@const isShort = visibleDuration <= SHORT_EVENT_DURATION_MINS}
    
    <div
        class="day-event"
        class:ev-plan={!!event.taskId}
        class:is-done={event.taskStatus === 'done'}
        class:is-compact={compact}
        class:is-short={isShort && !compact}
        class:is-ghost={isGhost}
        class:is-floating={isFloating}
        class:is-readonly={readOnly}
        class:is-selected={selected && !isGhost}
        style="
            top: {top}px;
            height: {height}px;
            --event-lane: {lane};
            --event-lanes: {lanes};
            {eventColorVars(resolveEventColor(event, calendars))}
        "
        onpointerdown={isGhost ? undefined : onBodyDown}
    >
        <div
            role="separator"
            tabindex="-1"
            class="day-event-handle day-event-handle-top"
            onpointerdown={isGhost || readOnly ? undefined : onResizeStart('top')}
        ></div>
        
        <div class="day-event-inner">
            <div class="day-event-title" style="display: flex; align-items: center; gap: 4px;">
                {#if event.taskId}
                    <TaskCircle
                        size="sm"
                        isDoneOrChecking={event.taskStatus === 'done'}
                        onclick={(e) => {
                            e.stopPropagation();
                            onToggleTask?.(event.taskId!);
                        }}
                    />
                {/if}
                {#if isRecurring}
                    <span style="flex-shrink: 0; font-size: 14px;">↻</span>
                {/if}
                <span class="day-event-title-text">{title}</span>
            </div>
            <div class="day-event-time">
                {formatHhMm(start)} – {formatHhMm(end)}
            </div>
        </div>
        
        <div
            role="separator"
            tabindex="-1"
            class="day-event-handle day-event-handle-bottom"
            onpointerdown={isGhost || readOnly ? undefined : onResizeStart('bottom')}
        ></div>
    </div>
{/snippet}

{#if dragOffset !== undefined}
    {@render renderBlock(startMins, endMins, true, false)}
    {@render renderBlock(startMins + dragOffset, endMins + dragOffset, false, true)}
{:else}
    {@render renderBlock(startMins, endMins, false, false)}
{/if}
