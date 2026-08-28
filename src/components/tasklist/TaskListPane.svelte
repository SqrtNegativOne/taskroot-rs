<script lang="ts">
    import type { AppTaskFilter } from '../../lib/bindings/AppTaskFilter.generated';
    import TaskRow from './TaskRow.svelte';
    import FilterButton from '../FilterButton.svelte';
    import SortButton from '../SortButton.svelte';
    import SearchBar from '../SearchBar.svelte';
    import type { AppTask, AppTaskStatus } from '../../lib/domain';
    import type { TaskPriority } from '../../lib/bindings/TaskPriority.generated';
    import './tasklist.css';
    import { useAutoQuery } from '../../lib/safeInvoke.svelte';
    import { slide } from 'svelte/transition';
    import { flip } from 'svelte/animate';

    let {
        onUpdateTask,
        filters = $bindable([]),
        sort = $bindable([] as import('../../lib/bindings/AppTaskSort.generated').AppTaskSort[]),
        onDragStart,
        activeDragId,
        onAddTask,
        onDeleteTask,
        onTaskClick,
        footer,
    }: {
        onUpdateTask?: (id: string, transform: (t: AppTask) => AppTask) => void;
        filters?: AppTaskFilter[];
        sort?: import('../../lib/bindings/AppTaskSort.generated').AppTaskSort[];
        onDragStart?: (e: PointerEvent | MouseEvent, task: AppTask) => void;
        activeDragId?: string;
        onAddTask: (defaults?: Partial<AppTask>) => void;
        onDeleteTask?: (id: string) => void;
        onTaskClick?: (task: AppTask) => void;
        footer?: import('svelte').Snippet;
    } = $props();

    let query = $state('');

    function updateTask(id: string, transform: (t: AppTask) => AppTask) {
        if (onUpdateTask) {
            onUpdateTask(id, transform);
        }
    }

    function deleteTask(id: string) {
        if (onDeleteTask) onDeleteTask(id);
    }
    
    // Note: event-based past due calculation is deferred since we don't have global events accessible easily here without context
    // We can assume empty for now.
    const pastDueTaskIds = new Set<string>();

    const TASK_QUERY_DEBOUNCE_MS = 150;

    import type { AppTaskColumnDef } from '../../lib/bindings/AppTaskColumnDef.generated';

    let tasksQuery = useAutoQuery<AppTask[]>('query_tasks', () => ({ filters, sort, query }), { debounceMs: TASK_QUERY_DEBOUNCE_MS });
    let schemaQuery = useAutoQuery<AppTaskColumnDef[]>('get_task_schema', () => ({}));
    
    let schema = $derived(schemaQuery.data ?? []);
    
    let filterColumns = $derived(schema.map(c => ({ id: c.id, label: c.label })));
    let sortColumns = $derived(schema.filter(c => c.sortable).map(c => ({ id: c.id, label: c.label })));

    function getFilterValues(colId: string): string[] {
        const col = schema.find(c => c.id === colId);
        if (col && typeof col.filter_type === 'object' && 'Enum' in col.filter_type) {
            return col.filter_type.Enum;
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
        const safeDefaults: Partial<AppTask> = {};
        for (const f of filters) {
            if (f.operator === 'is' && f.value != null) {
                const vals = Array.isArray(f.value) ? f.value : [f.value];
                if (vals.length === 1) {
                    if (f.column === 'status') safeDefaults.status = vals[0] as AppTaskStatus;
                    if (f.column === 'priority') safeDefaults.priority = Number(vals[0]) as TaskPriority;
                    if (f.column === 'tags') safeDefaults.tags = [{ id: crypto.randomUUID(), name: String(vals[0]) }];
                }
            }
        }
        onAddTask(safeDefaults);
    }
</script>

<aside class="task-pane">
    <header class="task-pane-hd">
        <div style="width: 100%; margin-bottom: 8px;">
            <SearchBar 
                placeholder="Search tasks..." 
                value={query}
                onchange={(q) => query = q}
            />
        </div>
        <div class="task-pane-controls" style="display: flex; gap: 8px; align-items: center; flex-wrap: wrap; width: 100%;">
            <FilterButton bind:filters columns={filterColumns} getValuesForColumn={getFilterValues} />
            <SortButton bind:sort sortOptions={sortColumns} />
            <button
                style="margin-left: auto; background: var(--bg-surface); border: 1px solid var(--border); color: var(--fg); border-radius: 4px; cursor: pointer; padding: 4px 6px; display: flex; align-items: center; justify-content: center;"
                title="Add Task"
                onclick={handleAddTask}
            >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
            </button>
        </div>
    </header>

    <!-- svelte-ignore a11y_no_static_element_interactions -->
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
                        {deleteTask}
                        isPastDue={pastDueTaskIds.has(item.task.id)}
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
