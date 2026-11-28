<script lang="ts">
    import { onMount, untrack } from 'svelte';
    import { store } from '../../lib/store.svelte';
    import DayTimeline from '../plan/day-timeline/DayTimeline.svelte';
    import { buildNewEvent } from '../../lib/domain';
    import { useActiveCalendars } from '../../lib/sharedQueries.svelte';
    import {
        createSidebarState,
        createSidebarWindowController,
        saveTabTop,
        SIDEBAR_PANEL_WIDTH,
        SIDEBAR_TAB_HEIGHT,
        SIDEBAR_TAB_WIDTH,
    } from './state.svelte';
    import './sidebar.css';

    const sidebar = createSidebarState();
    const windowCtrl = createSidebarWindowController();

    let isDragging = $state(false);
    let dragStartScreenY = 0;
    let dragStartTabTop = 0;

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
        sidebar.tabTop = windowCtrl.clampTabTop(dragStartTabTop + delta);
        windowCtrl.moveCollapsedTab(sidebar.tabTop);
    }

    function onPointerUp(e: PointerEvent) {
        if (!isDragging) return;
        const target = e.currentTarget as HTMLElement;
        target.releasePointerCapture(e.pointerId);
        isDragging = false;

        saveTabTop(sidebar.tabTop);

        if (Math.abs(e.screenY - dragStartScreenY) < 3) {
            void windowCtrl.toggle(sidebar.tabTop);
        }
    }

    onMount(() => {
        void windowCtrl.init();
        return () => {
            windowCtrl.dispose();
        };
    });

    $effect(() => {
        if (!sidebar.hydrated || !windowCtrl.monitorReady) return;
        sidebar.tabTop = sidebar.storedTabTop ?? windowCtrl.defaultTabTop();
        untrack(() => {
            void windowCtrl.syncBounds(sidebar.tabTop);
        });
    });

    const calendarsQuery = useActiveCalendars();

    function onAddEvent(date: Date, startMins?: number, endMins?: number): Promise<string | undefined> {
        const newEvent = buildNewEvent(date, {
            startMinutes: startMins,
            endMinutes: endMins,
        });
        return store
            .mutate('Failed to create event', () => store.addEvent(newEvent))
            .then((result) => (result.isOk() ? newEvent.id : undefined));
    }
</script>

<div
    class="sidebar-widget"
    class:open={windowCtrl.open}
    style="--tab-width: {SIDEBAR_TAB_WIDTH}px; --tab-height: {SIDEBAR_TAB_HEIGHT}px; --panel-width: {SIDEBAR_PANEL_WIDTH}px;"
>
    <button
        class="tab"
        style="margin-top: {windowCtrl.windowIsSmall ? 0 : sidebar.tabTop}px;"
        onpointerdown={onPointerDown}
        onpointermove={onPointerMove}
        onpointerup={onPointerUp}
        onpointercancel={onPointerUp}
        aria-label="Toggle day column"
    >
        <span class="arrow">{windowCtrl.open ? '›' : '‹'}</span>
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
