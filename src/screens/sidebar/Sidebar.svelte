<script lang="ts">
    import { onMount, untrack } from 'svelte';
    import { getCurrentWindow, currentMonitor, primaryMonitor } from '@tauri-apps/api/window';
    import { PhysicalSize, PhysicalPosition } from '@tauri-apps/api/dpi';
    import { store } from '../../lib/store.svelte';
    import DayTimeline from '../plan/day-timeline/DayTimeline.svelte';
    import { buildNewEvent } from '../../lib/domain';
    import { commands, useAutoQuery } from '../../lib/safeInvoke.svelte';
    import { createSidebarState, saveTabTop } from './state.svelte';
    import './sidebar.css';

    const TAB_WIDTH = 18;
    const TAB_HEIGHT = 52;
    const PANEL_WIDTH = 328;

    let open = $state(false);
    let windowIsSmall = $state(true);
    const sidebar = createSidebarState();

    let isDragging = $state(false);
    let monitorReady = $state(false);
    let dragStartScreenY = 0;
    let dragStartTabTop = 0;

    let sf = 1;
    let screenW = 0;
    let screenH = 0;
    let screenX = 0;
    let screenY = 0;

    async function updateWindowBounds(isOpen: boolean) {
        if (screenW === 0 || screenH === 0) return;
        const appWindow = getCurrentWindow();
        if (isOpen) {
            const widthPhysical = Math.round((TAB_WIDTH + PANEL_WIDTH) * sf);
            const posX = screenX + screenW - widthPhysical;
            await appWindow.setPosition(new PhysicalPosition(posX, screenY));
            await appWindow.setSize(new PhysicalSize(widthPhysical, screenH));
        } else {
            const widthPhysical = Math.round(TAB_WIDTH * sf);
            const heightPhysical = Math.round(TAB_HEIGHT * sf);
            const posX = screenX + screenW - widthPhysical;
            const posY = Math.round(screenY + (sidebar.tabTop * sf));
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
        sidebar.showNotes = !sidebar.showNotes;
    }

    function onPointerDown(e: PointerEvent) {
        if (e.button !== 0) return; // only left click
        const target = e.currentTarget as HTMLElement;
        target.setPointerCapture(e.pointerId);
        isDragging = true;
        dragStartScreenY = e.screenY;
        dragStartTabTop = sidebar.tabTop;
        e.preventDefault(); // prevent text selection
    }

    function onPointerMove(e: PointerEvent) {
        if (!isDragging) return;
        const delta = e.screenY - dragStartScreenY;
        let newTabTop = dragStartTabTop + delta;
        
        const maxTop = (screenH / sf) - TAB_HEIGHT;
        if (newTabTop < 0) newTabTop = 0;
        if (newTabTop > maxTop) newTabTop = maxTop;
        
        sidebar.tabTop = newTabTop;

        if (windowIsSmall) {
            // update OS window position live
            const widthPhysical = Math.round(TAB_WIDTH * sf);
            const posX = screenX + screenW - widthPhysical;
            const posY = Math.round(screenY + (sidebar.tabTop * sf));
            void getCurrentWindow().setPosition(new PhysicalPosition(posX, posY));
        }
    }

    function onPointerUp(e: PointerEvent) {
        if (!isDragging) return;
        const target = e.currentTarget as HTMLElement;
        target.releasePointerCapture(e.pointerId);
        isDragging = false;
        
        saveTabTop(sidebar.tabTop);
        
        if (Math.abs(e.screenY - dragStartScreenY) < 3) {
            void toggleSidebar();
        }
    }

    onMount(async () => {
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
                monitorReady = true;
            }
        } catch (err) {
            console.error("Failed to get monitor:", err);
        }
    });

    $effect(() => {
        if (!sidebar.hydrated || !monitorReady) return;
        sidebar.tabTop = sidebar.storedTabTop ?? (screenH / sf - TAB_HEIGHT) / 2;
        untrack(() => {
            void updateWindowBounds(open);
        });
    });

    const calendarsQuery = useAutoQuery(commands.getActiveCalendars);

    function onAddEvent(date: Date, startMins?: number, endMins?: number): Promise<string | undefined> {
        const newEvent = buildNewEvent(date, {
            startMinutes: startMins,
            endMinutes: endMins,
            calendars: calendarsQuery.data ?? [],
        });
        return store
            .mutate('Failed to create event', () => store.addEvent(newEvent))
            .then((result) => (result.isOk() ? newEvent.id : undefined));
    }
</script>

<div
    class="sidebar-widget"
    class:open
    style="--tab-width: {TAB_WIDTH}px; --tab-height: {TAB_HEIGHT}px; --panel-width: {PANEL_WIDTH}px;"
>
    <button 
        class="tab"
        style="margin-top: {windowIsSmall ? 0 : sidebar.tabTop}px;"
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
            <div class="calendar-wrap" class:half={sidebar.showNotes}>
                <DayTimeline
                    calendars={calendarsQuery.data ?? []}
                    {onAddEvent}
                    variant="sidebar"
                />
            </div>
            {#if sidebar.showNotes}
                <div class="notes-wrap">
                    <textarea
                        bind:value={sidebar.notes}
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