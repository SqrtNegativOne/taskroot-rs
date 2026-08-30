<script lang="ts">
    import { store, describeError } from '../../lib/store.svelte';
    import type { AppEvent, AppTask, AppCalendar } from '../../lib/domain';
    import type { Result } from 'neverthrow';
    import type { AppError } from '../../lib/safeInvoke.svelte';
    import { useAutoQuery } from '../../lib/safeInvoke.svelte';
    import { addDays, ymd } from '../../lib/time';

    import SplitPane from '../../components/SplitPane.svelte';
    import TaskListPane from '../../components/tasklist/TaskListPane.svelte';
    import InspectorPane from '../../components/inspector-pane/InspectorPane.svelte';
    import DateGrid from './date-grid/DateGrid.svelte';
    import DayTimeline from './day-timeline/DayTimeline.svelte';
    import RecurringActionModal, { type RecurringMode } from '../../components/RecurringActionModal.svelte';
    import type { DragState } from './day-timeline/types';
    let dragState = $state<DragState | undefined>(undefined);
    let inspectorState = $state<{ type: 'event' | 'task', id: string } | undefined>(undefined);
    let recurringModalOpen = $state(false);
    let recurringActionType = $state<"edit" | "delete">("edit");
    
    function handleRecurringConfirm(mode: RecurringMode) {
        recurringModalOpen = false;
        console.log("Confirmed recurring action:", mode);
    }

    function handleRecurringCancel() {
        recurringModalOpen = false;
    }

    function mutateOrReport(operation: string, mutation: () => Promise<Result<void, AppError>>): Promise<void> {
        return mutation().then((result) => {
            if (result.isErr()) {
                store.error = `${operation}: ${describeError(result.error)}`;
            }
        });
    }

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
        void mutateOrReport('Failed to create task', () => store.addTask(newTask));
        inspectorState = { type: 'task', id: newTask.id };
    }

    function onDeleteTask(id: string) {
        void mutateOrReport('Failed to delete task', () => store.deleteTask(id));
        if (inspectorState?.id === id) inspectorState = undefined;
    }

    const MS_PER_MINUTE = 60_000;

    const calendarsQuery = useAutoQuery<AppCalendar[]>('get_active_calendars', () => ({}));

    function onAddEvent(date: Date, startMins?: number, endMins?: number) {
        const dayStart = new Date(date.getFullYear(), date.getMonth(), date.getDate());
        
        const activeCalendars = calendarsQuery.data ?? [];
        const primaryCalendar = activeCalendars.find(c => c.isPrimary);
        const defaultCalendarId = primaryCalendar ? primaryCalendar.id : (activeCalendars.length > 0 ? activeCalendars[0].id : undefined);

        const newEvent = {
            id: crypto.randomUUID(),
            title: 'New Event',
            startTime: startMins !== undefined
                ? new Date(dayStart.getTime() + startMins * MS_PER_MINUTE).toISOString()
                : ymd(date),
            endTime: endMins !== undefined
                ? new Date(dayStart.getTime() + endMins * MS_PER_MINUTE).toISOString()
                : ymd(addDays(date, 1)),
            remoteId: undefined, remoteCollectionId: defaultCalendarId, taskId: undefined, description: undefined, rrule: undefined,
            exdates: undefined, recurringEventId: undefined, originalStartTime: undefined, cancelled: undefined,
            updatedAt: undefined, color: undefined, _deleted: undefined, etag: undefined,
            isAllDay: startMins === undefined,
        } as AppEvent;
        void mutateOrReport('Failed to create event', () => store.addEvent(newEvent));
        inspectorState = { type: 'event', id: newEvent.id };
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
                    style="height: 100%; display: flex; flex-direction: column;"
                    ondblclick={(e) => {
                        if (e.target instanceof Element && e.target.closest('.task-row')) {
                            // double click on task row
                        }
                    }}
                >
                    <TaskListPane
                        onDragStart={onTaskDragStart}
                        {onAddTask}
                        {onDeleteTask}
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
                    >
                        {#snippet pane1()}
                            <DateGrid
                                {dragState}
                                {onEventDragStart}
                                {onAddEvent}
                                {onEventClick}
                            />
                        {/snippet}

                        {#snippet pane2()}
                            <DayTimeline
                                {dragState}
                                setDragState={(ds: import('./day-timeline/types').DragState | undefined) => { dragState = ds; }}
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
        <RecurringActionModal
            isOpen={recurringModalOpen}
            actionType={recurringActionType}
            onConfirm={handleRecurringConfirm}
            onCancel={handleRecurringCancel}
        />
    {/if}
</main>
