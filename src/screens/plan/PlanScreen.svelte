<script lang="ts">
    import { store, describeError } from '../../lib/store.svelte';
    import type { AppEvent, AppTask } from '../../lib/domain';
    import type { AppTaskFilter } from '../../lib/bindings/AppTaskFilter.generated';
    import type { AppEventFilter } from '../../lib/bindings/AppEventFilter.generated';
    import { useTauriQuery, queryDependency } from '../../lib/safeInvoke.svelte';
    import type { Result } from 'neverthrow';
    import type { AppError } from '../../lib/safeInvoke.svelte';
    import { addDays, ymd } from '../../lib/time';

    import SplitPane from '../../components/SplitPane.svelte';
    import TaskListPane from '../../components/tasklist/TaskListPane.svelte';
    import FilterButton from '../../components/FilterButton.svelte';
    import InspectorPane from '../../components/inspector-pane/InspectorPane.svelte';
    import DateGrid from './date-grid/DateGrid.svelte';
    import DayTimeline from './day-timeline/DayTimeline.svelte';
    import RecurringActionModal, { type RecurringMode } from '../../components/RecurringActionModal.svelte';
    import { DateGridView } from './date-grid/constants';
    import type { DragState } from './day-timeline/types';

    const DATE_GRID_VIEWS: readonly DateGridView[] = Object.values(DateGridView);
    const VIEW_STORAGE_KEY = 'taskroot_dategrid_view';
    const TASK_FILTERS_KEY = 'taskroot_task_filters';
    const TASK_SORT_KEY = 'taskroot_task_sort';
    const DATEGRID_FILTERS_KEY = 'taskroot_dategrid_filters';
    const TIMELINE_FILTERS_KEY = 'taskroot_timeline_filters';

    function getStored<T>(key: string, def: T): T {
        try {
            const val = localStorage.getItem(key);
            return val ? JSON.parse(val) as T : def;
        } catch {
            return def;
        }
    }

    function storedView(): DateGridView {
        const raw = localStorage.getItem(VIEW_STORAGE_KEY);
        return DATE_GRID_VIEWS.find((candidate) => candidate === raw) ?? DateGridView.ThreeWeeks;
    }

    let view = $state<DateGridView>(storedView());
    let anchor = $state(new Date());
    let timelineDate = $state(new Date());

    // UI state — task list
    let query = $state('');
    let filters = $state<AppTaskFilter[]>(
        getStored(TASK_FILTERS_KEY, [{ column: 'status', operator: 'is not', value: ['done'] }])
    );
    let sort = $state(localStorage.getItem(TASK_SORT_KEY) ?? 'priority');

    // UI state — events
    let dateGridQuery = $state('');
    let dateGridFilters = $state<AppEventFilter[]>(getStored(DATEGRID_FILTERS_KEY, []));

    let timelineQuery = $state('');
    let timelineFilters = $state<AppEventFilter[]>(getStored(TIMELINE_FILTERS_KEY, []));

    $effect(() => {
        localStorage.setItem(VIEW_STORAGE_KEY, view);
        localStorage.setItem(TASK_FILTERS_KEY, JSON.stringify(filters));
        localStorage.setItem(TASK_SORT_KEY, sort);
        localStorage.setItem(DATEGRID_FILTERS_KEY, JSON.stringify(dateGridFilters));
        localStorage.setItem(TIMELINE_FILTERS_KEY, JSON.stringify(timelineFilters));
    });

    let dateGridEventsQuery = useTauriQuery<AppEvent[]>('get_filtered_events');
    let dateGridEvents = $derived(dateGridEventsQuery.data ?? store.events);

    let timelineEventsQuery = useTauriQuery<AppEvent[]>('get_filtered_events');
    let timelineEvents = $derived(timelineEventsQuery.data ?? store.events);

    $effect(() => {
        queryDependency(store.events);
        if (!store.loaded) return;
        void dateGridEventsQuery.execute({ filters: dateGridFilters, query: dateGridQuery });
        void timelineEventsQuery.execute({ filters: timelineFilters, query: timelineQuery });
    });

    // Hydrate events with tasks
    let hydratedDateGridEvents = $derived.by(() => {
        return dateGridEvents.map(ev => {
            if (ev.taskId) {
                const task = store.tasks.find(t => t.id === ev.taskId);
                return { ...ev, task };
            }
            return ev;
        });
    });

    let hydratedTimelineEvents = $derived.by(() => {
        return timelineEvents.map(ev => {
            if (ev.taskId) {
                const task = store.tasks.find(t => t.id === ev.taskId);
                return { ...ev, task };
            }
            return ev;
        });
    });

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
            remoteId: undefined, remoteCollectionId: undefined, taskId: undefined, description: undefined, rrule: undefined,
            exdates: undefined, recurringEventId: undefined, originalStartTime: undefined, cancelled: undefined,
            updatedAt: undefined, color: undefined, _deleted: undefined, etag: undefined,
        } as AppEvent;
        void mutateOrReport('Failed to create event', () => store.addEvent(newEvent));
        inspectorState = { type: 'event', id: newEvent.id };
    }

    function onResizeEvent(id: string, startTime: string, endTime: string) {
        void mutateOrReport('Failed to update event', () => store.updateEvent(id, ev => ({ ...ev, startTime, endTime })));
    }

    function onMoveEvent(id: string, startTime: string, endTime: string) {
        void mutateOrReport('Failed to update event', () => store.updateEvent(id, ev => ({ ...ev, startTime, endTime })));
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
                        tasks={store.tasks}
                        onUpdateTask={(id: string, transform: (task: AppTask) => AppTask) => {
                            void mutateOrReport('Failed to update task', () => store.updateTask(id, transform));
                        }}
                        bind:filters
                        bind:sort
                        {query}
                        setQuery={(q: string) => { query = q; }}
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
                    {#snippet dateGridFilterMenu()}
                        <FilterButton
                            bind:filters={dateGridFilters}
                            columns={[
                                { id: 'calendar', label: 'Calendar' }
                            ]}
                            getValuesForColumn={(col) => {
                                if (col === 'calendar') {
                                    const cals = new Set(store.events.map(e => e.remoteCollectionId).filter(Boolean));
                                    return Array.from(cals) as string[];
                                }
                                return [];
                            }}
                            align="right"
                        />
                    {/snippet}

                    {#snippet timelineFilterMenu()}
                        <FilterButton
                            bind:filters={timelineFilters}
                            columns={[
                                { id: 'calendar', label: 'Calendar' }
                            ]}
                            getValuesForColumn={(col) => {
                                if (col === 'calendar') {
                                    const cals = new Set(store.events.map(e => e.remoteCollectionId).filter(Boolean));
                                    return Array.from(cals) as string[];
                                }
                                return [];
                            }}
                            align="right"
                        />
                    {/snippet}

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
                                events={hydratedDateGridEvents}
                                filterMenu={dateGridFilterMenu}
                                today={new Date()}
                                {dragState}
                                {onEventDragStart}
                                onAddEvent={(d: Date) => { onAddEvent(d); }}
                                {onEventClick}
                            />
                        {/snippet}

                        {#snippet pane2()}
                            <DayTimeline
                                events={hydratedTimelineEvents}
                                filterMenu={timelineFilterMenu}
                                eventFilters={timelineFilters}
                                eventQuery={timelineQuery}
                                today={new Date()}
                                {timelineDate}
                                setTimelineDate={(d: Date) => { timelineDate = d; }}
                                {dragState}
                                setDragState={(ds: DragState | undefined) => { dragState = ds; }}
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
            updateTask={(id: string, t: (task: AppTask) => AppTask) => { void mutateOrReport('Failed to update task', () => store.updateTask(id, t)); }}
            updateEvent={(id: string, e: (ev: AppEvent) => AppEvent) => { void mutateOrReport('Failed to update event', () => store.updateEvent(id, e)); }}
            deleteTask={(id: string) => {
                void mutateOrReport('Failed to delete task', () => store.deleteTask(id));
            }}
            deleteEvent={(id: string) => {
                void mutateOrReport('Failed to delete event', () => store.deleteEvent(id));
                inspectorState = undefined;
            }}
        />
        <RecurringActionModal
            isOpen={recurringModalOpen}
            actionType={recurringActionType}
            onConfirm={handleRecurringConfirm}
            onCancel={handleRecurringCancel}
        />
    {/if}
</main>
