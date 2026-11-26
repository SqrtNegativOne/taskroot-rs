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
    }: {
        tasks: AppTask[];
        activeTask?: AppTask;
        customLabel?: string;
        onSelect: (taskId: string) => void;
        onStop: () => void;
        onSetCustom: (label: string) => void;
        onToggleDone: (task: AppTask) => void;
    } = $props();

    let search = $state('');
    let custom = $state(untrack(() => customLabel ?? ''));

    let visibleTasks = $derived.by(() => {
        const needle = search.trim().toLowerCase();
        if (needle === '') return tasks;
        return tasks.filter((task) => task.title.toLowerCase().includes(needle));
    });
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
    <div class="menu-search">
        <input
            type="search"
            placeholder="Search tasks..."
            aria-label="Search tasks"
            bind:value={search}
            autofocus
        />
    </div>

    <form
        class="custom-form"
        onsubmit={(event) => {
            event.preventDefault();
            onSetCustom(custom);
        }}
    >
        <input
            type="text"
            placeholder="Or type your own label..."
            aria-label="Custom label"
            bind:value={custom}
        />
        <button type="submit" class="custom-submit" disabled={custom.trim() === ''}>Track</button>
        {#if customLabel}
            <button
                type="button"
                class="custom-clear"
                aria-label="Clear custom label"
                title="Clear custom label"
                onclick={() => onSetCustom('')}
            >×</button>
        {/if}
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
            <p class="empty">{search.trim() === '' ? 'No tasks' : 'No matching tasks'}</p>
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
    </div>
</div>