<script lang="ts">
    import { onMount } from 'svelte';
    import { getCurrentWindow, currentMonitor, primaryMonitor } from '@tauri-apps/api/window';
    import { PhysicalSize, PhysicalPosition } from '@tauri-apps/api/dpi';
    import { store, describeError } from '../../lib/store.svelte';
    import DayTimeline from '../plan/day-timeline/DayTimeline.svelte';
    import { ymd, addDays } from '../../lib/time';
    import type { AppEvent } from '../../lib/domain';

    let open = $state(false);
    let windowIsSmall = $state(true);
    let showNotes = $state(false);
    let notesText = $state('');
    let today = $state(new Date());
    let timelineDate = $state(new Date());
    
    let tabTop = $state(0);
    let isDragging = $state(false);
    let dragStartScreenY = 0;
    let dragStartTabTop = 0;

    let hydratedEvents = $derived.by(() => {
        return store.events.map(ev => {
            if (ev.taskId) {
                const task = store.tasks.find(t => t.id === ev.taskId);
                return { ...ev, task };
            }
            return ev;
        });
    });

    let sf = 1;
    let screenW = 0;
    let screenH = 0;
    let screenX = 0;
    let screenY = 0;

    async function updateWindowBounds(isOpen: boolean) {
        if (screenW === 0 || screenH === 0) return;
        const appWindow = getCurrentWindow();
        if (isOpen) {
            const widthPhysical = Math.round(354 * sf); // 350 + 4px buffer
            const posX = screenX + screenW - widthPhysical;
            await appWindow.setPosition(new PhysicalPosition(posX, screenY));
            await appWindow.setSize(new PhysicalSize(widthPhysical, screenH));
        } else {
            const widthPhysical = Math.round(26 * sf); // 22 + 4px buffer
            const heightPhysical = Math.round(64 * sf);
            const posX = screenX + screenW - widthPhysical;
            const posY = Math.round(screenY + (tabTop * sf));
            await appWindow.setPosition(new PhysicalPosition(posX, posY));
            await appWindow.setSize(new PhysicalSize(widthPhysical, heightPhysical));
        }
    }

    async function toggleSidebar() {
        if (!open) {
            windowIsSmall = false;
            // Expand OS window first, then CSS animate
            await updateWindowBounds(true);
            open = true;
        } else {
            // CSS animate first, then shrink OS window
            open = false;
            setTimeout(() => {
                windowIsSmall = true;
                void updateWindowBounds(false);
            }, 250); // wait for 250ms CSS transition
        }
    }

    function toggleNotes() {
        showNotes = !showNotes;
    }

    function onPointerDown(e: PointerEvent) {
        if (e.button !== 0) return; // only left click
        const target = e.currentTarget as HTMLElement;
        target.setPointerCapture(e.pointerId);
        isDragging = true;
        dragStartScreenY = e.screenY;
        dragStartTabTop = tabTop;
        e.preventDefault(); // prevent text selection
    }

    function onPointerMove(e: PointerEvent) {
        if (!isDragging) return;
        const delta = e.screenY - dragStartScreenY;
        let newTabTop = dragStartTabTop + delta;
        
        const maxTop = (screenH / sf) - 64;
        if (newTabTop < 0) newTabTop = 0;
        if (newTabTop > maxTop) newTabTop = maxTop;
        
        tabTop = newTabTop;

        if (windowIsSmall) {
            // update OS window position live
            const widthPhysical = Math.round(26 * sf);
            const posX = screenX + screenW - widthPhysical;
            const posY = Math.round(screenY + (tabTop * sf));
            void getCurrentWindow().setPosition(new PhysicalPosition(posX, posY));
        }
    }

    function onPointerUp(e: PointerEvent) {
        if (!isDragging) return;
        const target = e.currentTarget as HTMLElement;
        target.releasePointerCapture(e.pointerId);
        isDragging = false;
        
        localStorage.setItem('sidebar_tab_top', tabTop.toString());
        
        if (Math.abs(e.screenY - dragStartScreenY) < 3) {
            void toggleSidebar();
        }
    }

    onMount(async () => {
        notesText = localStorage.getItem('sidebar_notes') ?? '';
        showNotes = localStorage.getItem('sidebar_show_notes') === 'true';

        const appWindow = getCurrentWindow();
        await appWindow.show();
        await appWindow.unminimize();
        await appWindow.setFocus();

        try {
            let monitor = await currentMonitor();
            monitor ??= await primaryMonitor();
            
            if (monitor) {
                sf = monitor.scaleFactor;
                screenW = monitor.size.width;
                screenH = monitor.size.height;
                screenX = monitor.position.x;
                screenY = monitor.position.y;
                
                const savedTabTop = localStorage.getItem('sidebar_tab_top');
                if (savedTabTop !== null) {
                    tabTop = parseFloat(savedTabTop);
                } else {
                    tabTop = (screenH / sf - 64) / 2;
                }
                
                await updateWindowBounds(open);
            }
        } catch (err) {
            console.error("Failed to get monitor:", err);
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
    <button 
        class="tab"
        style="margin-top: {windowIsSmall ? 0 : tabTop}px;"
        onpointerdown={onPointerDown}
        onpointermove={onPointerMove}
        onpointerup={onPointerUp}
        onpointercancel={onPointerUp}
        aria-label="Toggle day column"
    >
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
        padding-right: 4px;
    }

    .tab {
        align-self: flex-start;
        width: 22px;
        height: 64px;
        background: rgba(12, 12, 10, 0.75);
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
        background: rgba(12, 12, 10, 0.85);
    }
    .tab .arrow {
        transition: transform 200ms ease;
    }

    .panel {
        width: 0;
        min-width: 0;
        overflow: hidden;
        background: rgba(12, 12, 10, 0.75);
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
        background: transparent !important;
    }
    :global(.calendar-wrap .timeline-header) {
        border-radius: 0 !important;
        background: transparent !important;
    }

    .notes-wrap {
        height: 50%;
        border-top: 1px solid var(--border);
        display: flex;
        flex-direction: column;
        background: rgba(0, 0, 0, 0.2);
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
        background: rgba(12, 12, 10, 0.75);
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
        background: rgba(12, 12, 10, 0.85);
    }
</style>
