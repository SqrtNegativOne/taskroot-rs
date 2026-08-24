<script lang="ts">
    import { onMount } from 'svelte';
    import { getCurrentWindow, currentMonitor } from '@tauri-apps/api/window';
    import { LogicalSize, LogicalPosition } from '@tauri-apps/api/dpi';
    import { store, describeError } from '../../lib/store.svelte';
    import DayTimeline from '../plan/day-timeline/DayTimeline.svelte';
    import { ymd, addDays } from '../../lib/time';
    import type { AppEvent } from '../../lib/domain';

    let open = $state(true);
    let showNotes = $state(false);
    let notesText = $state('');
    let today = $state(new Date());
    let timelineDate = $state(new Date());

    let hydratedEvents = $derived.by(() => {
        return store.events.map(ev => {
            if (ev.taskId) {
                const task = store.tasks.find(t => t.id === ev.taskId);
                return { ...ev, task };
            }
            return ev;
        });
    });

    function toggleSidebar() {
        open = !open;
    }

    function toggleNotes() {
        showNotes = !showNotes;
    }

    onMount(async () => {
        notesText = localStorage.getItem('sidebar_notes') ?? '';
        showNotes = localStorage.getItem('sidebar_show_notes') === 'true';

        // Auto position to right edge
        const appWindow = getCurrentWindow();
        const monitor = await currentMonitor();
        if (monitor) {
            const sf = monitor.scaleFactor;
            const screenW = monitor.size.width;
            const screenH = monitor.size.height;
            const widthPhysical = Math.round(350 * sf);
            
            const posX = monitor.position.x + screenW - widthPhysical;
            const posY = monitor.position.y;
            
            await appWindow.setPosition(new LogicalPosition(posX / sf, posY / sf));
            await appWindow.setSize(new LogicalSize(350, screenH / sf));
            await appWindow.show();
        }
    });

    $effect(() => {
        localStorage.setItem('sidebar_notes', notesText);
        localStorage.setItem('sidebar_show_notes', String(showNotes));
    });

    const MS_PER_MINUTE = 60_000;
    
    function mutateOrReport(operation: string, mutation: () => Promise<import('neverthrow').Result<void, import('../../lib/safeInvoke.svelte').AppError>>): Promise<void> {
        return mutation().then((result) => {
            if (result.isErr()) {
                store.error = `${operation}: ${describeError(result.error)}`;
            }
        });
    }

    function onAddEvent(date: Date, startMins?: number, endMins?: number) {
        const dayStart = new Date(date.getFullYear(), date.getMonth(), date.getDate());
        const newEvent = {
            id: crypto.randomUUID(),
            title: 'New Event',
            startTime: startMins !== undefined
                ? new Date(dayStart.getTime() + startMins * MS_PER_MINUTE).toISOString()
                : ymd(date),
            endTime: endMins !== undefined
                ? new Date(dayStart.getTime() + endMins * MS_PER_MINUTE).toISOString()
                : ymd(addDays(date, 1)),
        } as AppEvent;
        void mutateOrReport('Failed to create event', () => store.addEvent(newEvent));
    }

    function onResizeEvent(id: string, startTime: string, endTime: string) {
        void mutateOrReport('Failed to update event', () => store.updateEvent(id, ev => ({ ...ev, startTime, endTime })));
    }

    function onMoveEvent(id: string, startTime: string, endTime: string) {
        void mutateOrReport('Failed to update event', () => store.updateEvent(id, ev => ({ ...ev, startTime, endTime })));
    }
</script>

<div class="widget" class:open>
    <button class="tab" onclick={toggleSidebar} aria-label="Toggle day column">
        <span class="arrow">{open ? '›' : '‹'}</span>
    </button>
    <div class="panel">
        {#if !store.loaded}
            <div class="panel-status">Connecting...</div>
        {:else if store.error}
            <div class="panel-status">Backend error</div>
        {:else}
            <div class="calendar-wrap" class:half={showNotes}>
                <DayTimeline
                    events={hydratedEvents}
                    {today}
                    {timelineDate}
                    setTimelineDate={(d: Date) => { timelineDate = d; }}
                    {onAddEvent}
                    {onResizeEvent}
                    {onMoveEvent}
                />
            </div>
            {#if showNotes}
                <div class="notes-wrap">
                    <textarea
                        bind:value={notesText}
                        placeholder="Type notes here..."
                        class="notes-textarea"
                    ></textarea>
                </div>
            {/if}
        {/if}
        <button class="notes-toggle-btn" onclick={toggleNotes} aria-label="Toggle notes" title="Toggle notes">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-square-pen"><path d="M12 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.375 2.625a1 1 0 0 1 3 3l-9.013 9.014a2 2 0 0 1-.853.505l-2.875.958.958-2.875a2 2 0 0 1 .506-.854z"/></svg>
        </button>
    </div>
</div>

<style>
    .widget {
        position: fixed;
        top: 0;
        right: 0;
        height: 100vh;
        display: flex;
        z-index: 1000;
        pointer-events: none;
        overflow: hidden;
        width: 100%;
        justify-content: flex-end;
    }

    .tab {
        align-self: center;
        width: 22px;
        height: 64px;
        background: var(--bg-surface);
        border: 1px solid var(--border);
        border-right: none;
        border-radius: 6px 0 0 6px;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        pointer-events: auto;
        color: var(--fg-muted);
        font-size: 14px;
        transition: color 150ms, background 150ms;
        flex-shrink: 0;
    }
    .tab:hover {
        color: var(--fg);
        background: var(--bg);
    }
    .tab .arrow {
        transition: transform 200ms ease;
    }

    .panel {
        width: 0;
        min-width: 0;
        overflow: hidden;
        background: rgba(12, 12, 10, 0.88);
        border-left: none;
        transition: width 250ms cubic-bezier(0.4, 0, 0.2, 1);
        pointer-events: none;
        display: flex;
        flex-direction: column;
        position: relative;
    }
    .open .panel {
        width: 328px; /* 350 total - 22 tab */
        border-left: 1px solid var(--border);
        pointer-events: auto;
    }

    .panel-status {
        flex: 1;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: 0.75rem;
        padding: 1.5rem;
        text-align: center;
        font-size: 0.8rem;
        color: var(--fg-muted);
    }

    .calendar-wrap {
        flex: 1;
        overflow: hidden;
        min-height: 0;
        transition: height 250ms cubic-bezier(0.4, 0, 0.2, 1);
        display: flex;
        flex-direction: column;
    }
    .calendar-wrap.half {
        flex: none;
        height: 50%;
    }
    
    :global(.calendar-wrap .day-pane) {
        flex: 1;
        height: 100%;
        border-radius: 0;
    }
    :global(.calendar-wrap .timeline-header) {
        border-radius: 0 !important;
    }

    .notes-wrap {
        height: 50%;
        border-top: 1px solid var(--border);
        display: flex;
        flex-direction: column;
        background: rgba(18, 18, 16, 0.95);
        min-height: 0;
    }

    .notes-textarea {
        flex: 1;
        background: transparent;
        border: none;
        color: var(--fg);
        padding: 12px;
        padding-bottom: 38px;
        padding-right: 38px;
        resize: none;
        font-size: 0.85rem;
        font-family: inherit;
        line-height: 1.5;
        outline: none;
        scrollbar-width: thin;
        scrollbar-color: var(--border) transparent;
    }

    .notes-toggle-btn {
        position: absolute;
        bottom: 0;
        right: 0;
        width: 32px;
        height: 32px;
        background: var(--bg-surface);
        border-top: 1px solid var(--border);
        border-left: 1px solid var(--border);
        border-bottom: none;
        border-right: none;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        color: var(--fg-muted);
        z-index: 100;
        transition: color 150ms, background 150ms;
        pointer-events: auto;
    }
    .notes-toggle-btn:hover {
        color: var(--fg);
        background: var(--bg);
    }
</style>
