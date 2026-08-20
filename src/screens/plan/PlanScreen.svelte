<script lang="ts">
    import { onMount } from 'svelte';
    import { store } from '../../lib/store.svelte';
    import type { AppTask, AppEvent } from '../../lib/domain';

    import SplitPane from '../../components/SplitPane.svelte';
    import TaskListPane from '../../components/tasklist/TaskListPane.svelte';
    import DateGrid from './date-grid/DateGrid.svelte';
    import DayTimeline from './day-timeline/DayTimeline.svelte';
    import InspectorPane from '../../components/inspector-pane/InspectorPane.svelte';
    import { DateGridView } from './date-grid/constants';

    
    $effect(() => {
        console.log("PlanScreen reactive update: loaded=", store.loaded, "error=", store.error);
    });

    onMount(() => {
        void store.init();
    });

    // We will patch the store's init method to also log to our visual console
    let view = $state<DateGridView>(DateGridView.Month);
    let anchor = $state(new Date());
    let timelineDate = $state(new Date());
    
    import type { AppFilter } from '../../lib/bindings/AppFilter';

    // UI state — task list
    let query = $state('');
    let filters = $state<AppFilter[]>([]);
    let sort = $state('priority');

    // Hydrate events with tasks
    let hydratedEvents = $derived.by(() => {
        return store.events.map(ev => {
            if (ev.taskId) {
                const task = store.tasks.find(t => t.id === ev.taskId);
                return { ...ev, task };
            }
            return ev;
        });
    });
    
    let dragState = $state<import('./day-timeline/types').DragState | undefined>(undefined);
    let inspectorState = $state<{ type: 'event' | 'task', id: string } | undefined>(undefined);
    
    function onTaskDragStart() {
        // Drag logic stub
    }
    
    function onEventDragStart() {
        // Drag logic stub
    }
    
    function onAddTask(defaults?: Partial<AppTask>) {
        const newTask = {
            id: crypto.randomUUID(),
            title: 'New Task',
            status: 'todo' as const,
            priority: undefined, tags: undefined, subtasks: undefined, parentTask: undefined,
            dependencies: undefined, est: undefined, added: undefined, canvasX: undefined, canvasY: undefined,
            onCanvas: undefined, remoteId: undefined, notes: undefined, tabs: undefined, due: undefined,
            _deleted: undefined, updatedAt: undefined, etag: undefined,
            ...defaults
        } as AppTask;
        void store.addTask(newTask);
    }
    
    function onDeleteTask(id: string) {
        void store.deleteTask(id);
        if (inspectorState?.id === id) inspectorState = undefined;
    }
    
    function onAddEvent(date: Date, startMins?: number, endMins?: number) {
        const dStr = date.toISOString().split('T')[0];
        const newEvent = {
            id: crypto.randomUUID(),
            title: 'New Event',
            type: 'plan' as const,
            startTime: startMins ? new Date(new Date(dStr).getTime() + startMins * 60000).toISOString() : dStr,
            endTime: endMins ? new Date(new Date(dStr).getTime() + endMins * 60000).toISOString() : new Date(new Date(dStr).getTime() + 86400000).toISOString(),
            remoteId: undefined, remoteCollectionId: undefined, taskId: undefined, description: undefined, rrule: undefined,
            exdates: undefined, recurringEventId: undefined, originalStartTime: undefined, cancelled: undefined,
            updatedAt: undefined, color: undefined, _deleted: undefined, etag: undefined,
        } as AppEvent;
        void store.addEvent(newEvent);
    }
    
    function onResizeEvent(id: string, startTime: string, endTime: string) {
        void store.updateEvent(id, ev => ({ ...ev, startTime, endTime }));
    }
    
    function onMoveEvent(id: string, startTime: string, endTime: string) {
        void store.updateEvent(id, ev => ({ ...ev, startTime, endTime }));
    }

    function onEventClick(ev: AppEvent) {
        inspectorState = { type: 'event', id: ev.id };
    }
</script>

<main class="main" style="position: relative; height: 100vh; display: flex; flex-direction: column;">
    {#if store.error}
        <div style="padding: 20px; color: var(--danger); font-family: monospace;">
            Error loading data from backend: {store.error}
        </div>
    {:else if !store.loaded}
        <div style="padding: 20px;">Loading...</div>
    {:else}
        <SplitPane
            direction="horizontal"
            defaultSize={360}
            minSize={200}
            snapThreshold={50}
        >
            {#snippet pane1()}
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div 
                    style="height: 100%;" 
                    ondblclick={(e) => {
                        if (e.target instanceof Element && e.target.closest('.task-row')) {
                            // double click on task row
                        }
                    }}
                >
                    <TaskListPane
                        tasks={store.tasks}
                        onUpdateTask={(id, transform) => {
                            void store.updateTask(id, transform);
                        }}
                        bind:filters
                        bind:sort
                        {query}
                        setQuery={(q: string) => { query = q; }}
                        onDragStart={onTaskDragStart}
                        {onAddTask}
                        {onDeleteTask}
                    />
                </div>
            {/snippet}

            {#snippet pane2()}
                <div class="right-pane" style="height: 100%; display: flex; flex-direction: column;">
                    <SplitPane
                        direction="vertical"
                        defaultSize={450}
                        minSize={150}
                        snapThreshold={60}
                    >
                        {#snippet pane1()}
                            <DateGrid
                                {view}
                                setView={(v: DateGridView) => { view = v; }}
                                {anchor}
                                setAnchor={(a: Date) => { anchor = a; }}
                                events={hydratedEvents}
                                today={new Date()}
                                {dragState}
                                {onEventDragStart}
                                onAddEvent={(d: Date) => { onAddEvent(d); }}
                            />
                        {/snippet}
                        
                        {#snippet pane2()}
                            <DayTimeline
                                events={hydratedEvents}
                                today={new Date()}
                                {timelineDate}
                                setTimelineDate={(d: Date) => { timelineDate = d; }}
                                {dragState}
                                setDragState={(ds: import('./day-timeline/types').DragState | undefined) => { dragState = ds; }}
                                {onResizeEvent}
                                {onMoveEvent}
                                {onEventClick}
                                onAddEvent={(d: Date, s: number, e: number) => { onAddEvent(d, s, e); }}
                            />
                        {/snippet}
                    </SplitPane>
                </div>
            {/snippet}
        </SplitPane>
        <InspectorPane
            {inspectorState}
            onClose={() => { inspectorState = undefined; }}
            tasks={store.tasks}
            events={store.events}
            updateTask={(id: string, t: (task: AppTask) => AppTask) => { void store.updateTask(id, t); }}
            updateEvent={(id: string, e: (ev: AppEvent) => AppEvent) => { void store.updateEvent(id, e); }}
            deleteTask={(id: string) => {
                void store.deleteTask(id);
            }}
            deleteEvent={(id: string) => {
                void store.deleteEvent(id);
                inspectorState = undefined;
            }}
        />
    {/if}
</main>
