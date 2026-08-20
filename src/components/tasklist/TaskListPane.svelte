<script lang="ts">
    import type { AppFilter } from '../../lib/bindings/AppFilter';
    import TaskRow from './TaskRow.svelte';
    import FilterSortButtons from './FilterSortButtons.svelte';
    import type { AppTask } from '../../lib/domain';
    import './tasklist.css';
    import { useTauriQuery, safeInvoke } from '../../lib/safeInvoke.svelte';
    import type { AppTaskDefaults } from '../../lib/bindings/AppTaskDefaults';
    import { slide } from 'svelte/transition';
    import { flip } from 'svelte/animate';

    let {
        tasks,
        onUpdateTask,
        filters = $bindable([]),
        sort = $bindable('priority'),
        query,
        setQuery,
        onDragStart,
        activeDragId,
        onAddTask,
        onDeleteTask,
        footer,
    }: {
        tasks: AppTask[];
        onUpdateTask?: (id: string, transform: (t: AppTask) => AppTask) => void;
        filters?: AppFilter[];
        sort?: string;
        query: string;
        setQuery: (q: string) => void;
        onDragStart?: (e: PointerEvent | MouseEvent, task: AppTask) => void;
        activeDragId?: string;
        onAddTask: (defaults?: Partial<AppTask>) => void;
        onDeleteTask?: (id: string) => void;
        footer?: import('svelte').Snippet;
    } = $props();

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

    let tasksQuery = useTauriQuery<AppTask[]>('get_filtered_tasks');
    let filtered = $derived(tasksQuery.data ?? []);

    $effect(() => {
        // Trigger fetch when tasks, filters, sort, or query changes
        void tasks;
        tasksQuery.execute({ filters, sort, query });
    });

    async function handleAddTask() {
        const result = await safeInvoke<AppTaskDefaults>('compute_filter_defaults', { filters });
        result.match(
            (defaults) => {
                const safeDefaults: Partial<AppTask> = {};
                if (defaults.status !== null) safeDefaults.status = defaults.status;
                if (defaults.priority !== null) safeDefaults.priority = defaults.priority;
                if (defaults.tags !== null) safeDefaults.tags = defaults.tags;
                onAddTask(safeDefaults);
            },
            () => onAddTask()
        );
    }
</script>

<aside class="task-pane">
    <header class="task-pane-hd">
        <input 
            type="text" 
            placeholder="Search tasks..." 
            value={query}
            oninput={(e) => { setQuery(e.currentTarget.value); }}
            style="width: 100%; margin-bottom: 8px; padding: 4px 8px;"
        />
        <div class="task-pane-controls" style="display: flex; gap: 8px; align-items: center; flex-wrap: wrap; width: 100%;">
            <FilterSortButtons bind:filters bind:sort />
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
                <span class="dim">{tasks.length === 0 ? "no tasks exist." : "no tasks match."}</span>
            </div>
        {:else}
            {#each filtered as t (t.id)}
                <div animate:flip={{ duration: 300 }} transition:slide={{ duration: 250 }}>
                    <TaskRow
                        task={t}
                        {onDragStart}
                        dragging={activeDragId === t.id}
                        {updateTask}
                        {deleteTask}
                        isPastDue={pastDueTaskIds.has(t.id)}
                    />
                </div>
            {/each}
        {/if}
    </div>

    {#if footer}
        {@render footer()}
    {/if}
</aside>
