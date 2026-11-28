<script lang="ts">
    import { untrack } from 'svelte';
    import type { AppTask } from '../../lib/domain';
    import './task-menu.css';

    let {
        tasks,
        activeTask,
        customLabel,
        onSelect,
        onStop,
        onSetCustom,
        onToggleDone,
        onAddTask,
    }: {
        tasks: AppTask[];
        activeTask?: AppTask;
        customLabel?: string;
        onSelect: (taskId: string) => void;
        onStop: () => void;
        onSetCustom: (label: string) => void;
        onToggleDone: (task: AppTask) => void;
        onAddTask: (title: string) => void;
    } = $props();

    let query = $state(untrack(() => customLabel ?? ''));
    let trimmedQuery = $derived(query.trim());

    let visibleTasks = $derived.by(() => {
        const needle = trimmedQuery.toLowerCase();
        if (needle === '') return tasks;
        return tasks.filter((task) => task.title.toLowerCase().includes(needle));
    });

    let showAddTask = $derived(trimmedQuery.length > 0 && visibleTasks.length <= 2);
</script>

<div
    class="task-menu"
    data-task-menu
    role="menu"
    aria-label="Select a task"
    tabindex="-1"
    onpointerdown={(e) => e.stopPropagation()}
    ondblclick={(e) => e.stopPropagation()}
>
    <form
        class="menu-input"
        onsubmit={(event) => {
            event.preventDefault();
            onSetCustom(query);
        }}
    >
        <div class="search-field">
            <span class="search-icon material-symbols-outlined" aria-hidden="true">search</span>
            <input
                type="search"
                aria-label="Search tasks or type a label to track"
                bind:value={query}
                autofocus
            />
        </div>
        <button
            type="submit"
            class="track-button"
            aria-label="Track typed label"
            title="Track typed label"
        >
            <span class="material-symbols-outlined" aria-hidden="true">track_changes</span>
        </button>
    </form>

    {#if activeTask}
        <div class="selected-row">
            <span class="selected-title" title={activeTask.title}>{activeTask.title}</span>
            <button
                type="button"
                class="task-check"
                aria-label={`Mark ${activeTask.title} done`}
                title="Mark done"
                onclick={() => onToggleDone(activeTask)}
            >
                <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                    <path d="M20 6 9 17l-5-5" />
                </svg>
            </button>
            <button
                type="button"
                class="selected-clear"
                aria-label="Stop working"
                title="Stop working"
                onclick={onStop}
            >×</button>
        </div>
    {/if}

    <div class="menu-list">
        {#if visibleTasks.length === 0}
            <p class="empty" class:compact={showAddTask}>{trimmedQuery === '' ? 'No tasks' : 'No matching tasks'}</p>
        {:else}
            <ul>
                {#each visibleTasks as task (task.id)}
                    <li class="task-item">
                        <button
                            type="button"
                            role="menuitem"
                            class="task-row"
                            class:active={task.id === activeTask?.id}
                            onclick={() => onSelect(task.id)}
                            title={task.title}
                        >
                            <span class="task-title">{task.title}</span>
                            {#if task.id === activeTask?.id}
                                <span class="task-badge">current</span>
                            {/if}
                        </button>
                        <button
                            type="button"
                            class="task-check"
                            class:is-done={task.status === 'done'}
                            aria-label={task.status === 'done' ? `Mark ${task.title} not done` : `Mark ${task.title} done`}
                            title={task.status === 'done' ? 'Mark not done' : 'Mark done'}
                            onclick={() => onToggleDone(task)}
                        >
                            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                                <path d="M20 6 9 17l-5-5" />
                            </svg>
                        </button>
                    </li>
                {/each}
            </ul>
        {/if}
        {#if showAddTask}
            <div class="add-task-wrap">
                <button
                    type="button"
                    class="add-task-button"
                    onclick={() => onAddTask(trimmedQuery)}
                    title={`Add task "${trimmedQuery}"`}
                >
                    <span class="add-task-icon material-symbols-outlined" aria-hidden="true">add</span>
                    <span class="add-task-label">Add task</span>
                    <span class="add-task-preview">“{trimmedQuery}”</span>
                </button>
            </div>
        {/if}
    </div>
</div>