<script lang="ts">
    import { untrack } from 'svelte';
    import TaskRow from './TaskRow.svelte';
    import TaskContextMenu from './TaskContextMenu.svelte';
    import ViewMenuButton from '../ViewMenuButton.svelte';
    import SearchBar from '../SearchBar.svelte';
    import {
        calendarsForComponent,
        computeFilterDefaults,
        type AppCalendar,
        type AppTask,
        type TaskPatch,
    } from '../../lib/domain';
    import { taskListQueryArgs, taskListState } from '../../lib/taskList.svelte';
    import './tasklist.css';
    import { commands, useAutoQuery } from '../../lib/safeInvoke.svelte';
    import { slide } from 'svelte/transition';
    import { flip } from 'svelte/animate';

    let {
        calendars: propCalendars,
        onUpdateTask,
        onDragStart,
        activeDragId,
        onAddTask,
        onDeleteTask,
        onTaskClick,
        selectedTaskId,
        activeTaskId,
        onStartTask,
        footer,
    }: {
        calendars?: AppCalendar[];
        onUpdateTask?: (id: string, patches: TaskPatch[]) => void;
        onDragStart?: (e: PointerEvent | MouseEvent, task: AppTask) => void;
        activeDragId?: string;
        onAddTask: (defaults?: Partial<AppTask>) => void;
        onDeleteTask?: (id: string) => void;
        onTaskClick?: (task: AppTask) => void;
        selectedTaskId?: string;
        activeTaskId?: string;
        onStartTask?: (task: AppTask) => void;
        footer?: import('svelte').Snippet;
    } = $props();

    let query = $state('');

    function updateTask(id: string, patches: TaskPatch[]) {
        onUpdateTask?.(id, patches);
    }

    let contextMenu = $state<{ task: AppTask; x: number; y: number } | null>(null);

    function openContextMenu(e: MouseEvent, task: AppTask) {
        contextMenu = { task, x: e.clientX, y: e.clientY };
    }
    
    let pastDueQuery = useAutoQuery(commands.getPastDueTaskIds);
    let pastDueTaskIds = $derived(new Set(pastDueQuery.data ?? []));

    const ownCalendarsQuery = untrack(() => propCalendars === undefined)
        ? useAutoQuery(commands.getActiveCalendars)
        : undefined;
    let allCalendars = $derived(propCalendars ?? ownCalendarsQuery?.data ?? []);

    const TASK_QUERY_DEBOUNCE_MS = 150;

    let tasksQuery = useAutoQuery(commands.queryTasks, () => taskListQueryArgs(query), { debounceMs: TASK_QUERY_DEBOUNCE_MS });
    let schemaQuery = useAutoQuery(commands.getTaskSchema);
    
    let schema = $derived(schemaQuery.data ?? []);
    
    let filterColumns = $derived(schema.map(c => ({ id: c.id, label: c.label, filterType: c.filter_type })));
    let sortColumns = $derived(schema.filter(c => c.sortable).map(c => ({ id: c.id, label: c.label })));

    function getFilterValues(colId: string): {label: string, value: string}[] {
        if (colId === 'remotecollectionid') {
            return calendarsForComponent(allCalendars, 'VTODO')
                .map(c => ({ label: c.summary || c.id, value: c.id }));
        }
        const col = schema.find(c => c.id === colId);
        if (col && typeof col.filter_type === 'object' && 'Enum' in col.filter_type && col.filter_type.Enum) {
            return col.filter_type.Enum.map(v => ({ label: v, value: v }));
        }
        return [];
    }

    let filtered = $derived(tasksQuery.data ?? []);

    interface FlatTaskTreeItem {
        task: AppTask;
        depth: number;
    }

    let flatNestedFiltered = $derived.by(() => {
        interface TaskTreeItem {
            task: AppTask;
            children: TaskTreeItem[];
        }

        const map: Record<string, TaskTreeItem> = {};
        for (const t of filtered) {
            map[t.id] = { task: t, children: [] };
        }
        
        const rootItems: TaskTreeItem[] = [];
        
        for (const t of filtered) {
            const item = map[t.id]!;
            if (t.parentTask && map[t.parentTask]) {
                map[t.parentTask]!.children.push(item);
            } else {
                rootItems.push(item);
            }
        }

        const result: FlatTaskTreeItem[] = [];
        function traverse(item: TaskTreeItem, depth: number) {
            result.push({ task: item.task, depth });
            for (const child of item.children) {
                traverse(child, depth + 1);
            }
        }
        for (const rootItem of rootItems) {
            traverse(rootItem, 0);
        }
        return result;
    });

    function handleAddTask() {
        const rawDefaults = computeFilterDefaults(taskListState.filters);
        const safeDefaults: Partial<AppTask> = {
            status: rawDefaults.status,
            priority: rawDefaults.priority,
            tags: Array.isArray(rawDefaults.tags)
                ? rawDefaults.tags.map((t) => ({ id: crypto.randomUUID(), name: String(t) }))
                : undefined,
        };
        onAddTask(safeDefaults);
    }
</script>

<aside class="task-pane">
    <header class="task-pane-hd">
        <div class="task-pane-search-row">
            <SearchBar
                value={query}
                onchange={(q) => query = q}
            />
            <ViewMenuButton
                bind:filters={taskListState.filters}
                columns={filterColumns}
                getValuesForColumn={getFilterValues}
                bind:sort={taskListState.sort}
                sortOptions={sortColumns}
            />
            <button
                class="task-add-btn"
                title="Add Task"
                onclick={handleAddTask}
            >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
            </button>
        </div>
    </header>

    <div
        class="task-list"
        ondblclick={(e) => {
            if (!(e.target instanceof Element)) return;
            if (!e.target.closest('.task-row') && !e.target.closest('button')) {
                handleAddTask();
            }
        }}
    >
        {#if filtered.length === 0}
            <div class="task-empty">
                <span class="dim">no tasks match.</span>
            </div>
        {:else}
            {#each flatNestedFiltered as item (item.task.id)}
                <div animate:flip={{ duration: 300 }} transition:slide={{ duration: 250 }} style="margin-left: {item.depth * 20}px">
                    <TaskRow
                        task={item.task}
                        {onDragStart}
                        dragging={activeDragId === item.task.id}
                        {updateTask}
                        isPastDue={pastDueTaskIds.has(item.task.id)}
                        selected={item.task.id === selectedTaskId}
                        isActive={item.task.id === activeTaskId}
                        onContextMenu={openContextMenu}
                        {onTaskClick}
                    />
                </div>
            {/each}
        {/if}
    </div>

    {#if footer}
        {@render footer()}
    {/if}
</aside>

{#if contextMenu}
    <TaskContextMenu
        task={contextMenu.task}
        x={contextMenu.x}
        y={contextMenu.y}
        onClose={() => { contextMenu = null; }}
        onDelete={(id) => { onDeleteTask?.(id); }}
        onDo={(task) => { onStartTask?.(task); }}
    />
{/if}


