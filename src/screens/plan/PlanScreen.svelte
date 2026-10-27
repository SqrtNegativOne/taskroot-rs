<script lang="ts">
    import { store } from '../../lib/store.svelte';
    import { persistedKeys } from '../../lib/persisted.svelte';
    import type { AppTask, AppCalendar, EventInstance } from '../../lib/domain';
    import { buildNewEvent } from '../../lib/domain';
    import { useAutoQuery } from '../../lib/safeInvoke.svelte';

    import SplitPane from '../../components/SplitPane.svelte';
    import TaskListPane from '../../components/tasklist/TaskListPane.svelte';
    import InspectorPane from '../../components/inspector-pane/InspectorPane.svelte';
    import DateGrid from './date-grid/DateGrid.svelte';
    import DayTimeline from './day-timeline/DayTimeline.svelte';
    import { createEventDragController } from './hooks/eventDrag.svelte';
    import { createTaskDragController } from './hooks/taskDrag.svelte';
    import type { DragState } from './day-timeline/types';
    let dragState = $state<DragState | undefined>(undefined);

    const startEventDrag = createEventDragController({
        setDragState: (ds) => { dragState = ds; },
        onReschedule: (event, payload) => {
            void store.mutate('Failed to move event', () =>
                store.rescheduleEvent(event.id, payload.startTime, payload.endTime, payload.isAllDay),
            );
        },
    });
    let inspectorState = $state<{ type: 'event' | 'task', id: string } | undefined>(undefined);

    const calendarsQuery = useAutoQuery<AppCalendar[]>('get_active_calendars', () => ({}));

    let defaultTaskCollection = $derived(
        calendarsQuery.data?.find((c) => c.component === 'VTODO' && c.writable)?.id
    );

    const startTaskDrag = createTaskDragController({
        setDragState: (ds) => { dragState = ds; },
        defaultDuration: () => store.settings?.default_task_duration || 30,
        onSchedule: (task, slot) => {
            const event = buildNewEvent(new Date(`${slot.date}T00:00:00`), {
                startMinutes: slot.minute,
                endMinutes: slot.minute + slot.duration,
                calendars: calendarsQuery.data ?? [],
                title: task.title,
                taskId: task.id,
            });
            void store
                .mutate('Failed to schedule task', () => store.addEvent(event))
                .then((result) => {
                    if (result.isOk()) inspectorState = { type: 'event', id: event.id };
                });
        },
    });

    async function onAddTask(defaults?: Partial<AppTask>) {
        const newTask: AppTask = {
            id: crypto.randomUUID(),
            title: 'New Task',
            status: 'todo',
            remoteCollectionId: defaultTaskCollection,
            ...defaults
        };
        const result = await store.mutate('Failed to create task', () => store.addTask(newTask));
        if (result.isOk()) inspectorState = { type: 'task', id: newTask.id };
    }

    function onDeleteTask(id: string) {
        void store.mutate('Failed to delete task', () => store.deleteTask(id));
        if (inspectorState?.id === id) inspectorState = undefined;
    }

    async function onAddEvent(date: Date, startMins?: number, endMins?: number) {
        const newEvent = buildNewEvent(date, {
            startMinutes: startMins,
            endMinutes: endMins,
            calendars: calendarsQuery.data ?? [],
        });
        const result = await store.mutate('Failed to create event', () => store.addEvent(newEvent));
        if (result.isOk()) inspectorState = { type: 'event', id: newEvent.id };
    }

    function onEventClick(ev: EventInstance) {
        inspectorState = { type: 'event', id: ev.id };
    }
</script>

<main class="main" style="position: relative; height: 100vh; display: flex; flex-direction: column;">

    {#if store.error}
        <div style="padding: 20px; color: var(--tag-red); font-family: monospace;">
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
            persistKey={persistedKeys.planTaskPaneSize}
        >
            {#snippet pane1()}
                <div style="height: 100%; display: flex; flex-direction: column;">
                    <TaskListPane
                        onDragStart={startTaskDrag}
                        {onAddTask}
                        {onDeleteTask}
                        onUpdateTask={(task: AppTask) => {
                            void store.mutate('Failed to update task', () => store.updateTask(task));
                        }}
                        onTaskClick={(task: AppTask) => {
                            inspectorState = { type: 'task', id: task.id };
                        }}
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
                        persistKey={persistedKeys.planCalendarSplitSize}
                    >
                        {#snippet pane1()}
                            <DateGrid
                                {dragState}
                                onEventDragStart={startEventDrag}
                                {onAddEvent}
                                {onEventClick}
                            />
                        {/snippet}

                        {#snippet pane2()}
                            <DayTimeline
                                {dragState}
                                {onEventClick}
                                {onAddEvent}
                            />
                        {/snippet}
                    </SplitPane>
                </div>
            {/snippet}
        </SplitPane>
        <InspectorPane
            {inspectorState}
            onClose={() => { inspectorState = undefined; }}
        />
    {/if}
</main>
