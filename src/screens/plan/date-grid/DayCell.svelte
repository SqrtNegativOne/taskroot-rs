<script lang="ts">
    import type { AppEvent } from '../../../lib/domain';

    const OPACITY_FADED = 0.4;

    let {
        cell,
        today,
        events,
        isWeek,
        dragState,
        onEventDragStart,
        onAddEvent,
    }: {
        cell: { date: Date; outOfMonth: boolean };
        today: Date;
        events: AppEvent[];
        isWeek: boolean;
        dragState?: { target?: { kind: string; date?: string }; event?: { id: string } };
        onEventDragStart?: (e: PointerEvent, ev: AppEvent) => void;
        onAddEvent?: (date: Date) => void;
    } = $props();

    function ymd(d: Date) {
        return d.toISOString().split('T')[0];
    }
    
    function sameDay(a: Date, b: Date) {
        return ymd(a) === ymd(b);
    }
    
    function extractHourMinuteFromISO(iso: string) {
        const d = new Date(iso);
        return `${d.getHours().toString()}:${d.getMinutes().toString().padStart(2, '0')}`;
    }

    let isToday = $derived(sameDay(cell.date, today));
    let isPast = $derived(cell.date < today && !isToday);
    let cellDateStr = $derived(ymd(cell.date));
    let isDragOver = $derived(dragState?.target?.kind === "grid-day" && dragState.target.date === cellDateStr);
    let canAccept = $derived(!!dragState);

    // Animation logic
    let displayEvents = $derived(events);

    function isEventAllDay(e: AppEvent) {
        return e.type === 'plan' && !e.startTime;
    }
    
    function checkPastDue(ev: AppEvent): boolean {
        if (!ev.taskId) return false;
        return new Date(ev.endTime).getTime() < Date.now();
    }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    data-drop-kind="grid-day"
    data-drop-date={cellDateStr}
    class="day-cell"
    class:is-out={cell.outOfMonth}
    class:is-today={isToday}
    class:is-past={isPast}
    class:is-strip={isWeek}
    class:is-drag-over={isDragOver}
    class:can-accept={canAccept}
    ondblclick={(e) => {
        if (!(e.target instanceof HTMLElement)) return;
        if (e.target.closest(".day-cell-event")) return;
        if (onAddEvent) onAddEvent(cell.date);
    }}
>
    <div class="day-cell-hd">
        <span class="day-cell-num">
            {cell.date.getDate().toString().padStart(2, '0')}
        </span>
    </div>
    <div class="day-cell-events">
        {#each displayEvents as ev (ev.id)}
            {@const title = ev.title}
            {@const isDone = false /* TODO: pull done from task */}
            {@const isPastDue = checkPastDue(ev)}
            {@const isAllDay = isEventAllDay(ev)}
            
            <div
                class="day-cell-event ev-{ev.type}"
                class:is-done={isDone}
                title="{isAllDay ? 'All Day' : extractHourMinuteFromISO(ev.startTime)} — {title}"
                style="
                    cursor: grab;
                    opacity: {dragState?.event?.id === ev.id ? OPACITY_FADED : 1};
                    {ev.color ? `background-color: ${ev.color}; border-left-color: ${ev.color};` : ''}
                "
                onpointerdown={(e) => onEventDragStart?.(e, ev)}
            >
                {#if !isAllDay}
                    <span class="day-cell-event-time">
                        {extractHourMinuteFromISO(ev.startTime)}
                    </span>
                {/if}
                <span class="day-cell-event-title" style="display: flex; align-items: center; gap: 2px;">
                    {#if isPastDue}
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="flex-shrink: 0; color: var(--p0);">
                            <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
                        </svg>
                    {/if}
                    {title}
                </span>
            </div>
        {/each}
    </div>
    
    {#if isDragOver}
        <div class="day-cell-drop-hint">
            <span class="bracket">▸</span> drop to plan
        </div>
    {/if}
</div>
