<script lang="ts">
    import type { AppTask } from '../../lib/domain';

    let {
        tasks,
        activeTask,
        onSelect,
        onStop,
        onToggleDone,
    }: {
        tasks: AppTask[];
        activeTask?: AppTask;
        onSelect: (taskId: string) => void;
        onStop: () => void;
        onToggleDone: (task: AppTask) => void;
    } = $props();

    let search = $state('');

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

    {#if activeTask}
        <button type="button" role="menuitem" class="selected-row" onclick={onStop} title="Stop working on this task">
            <span class="selected-title">{activeTask.title}</span>
            <span class="selected-clear">×</span>
        </button>
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
                                <span class="task-badge">doing</span>
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

<style>
    .task-menu {
        display: flex;
        flex: 1;
        flex-direction: column;
        min-height: 0;
        overflow: hidden;
        background: rgba(0, 0, 0, 0.72);
        border-top: 1px solid rgba(255, 255, 255, 0.14);
    }

    .menu-search {
        flex: none;
        padding: 6px 8px;
        border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    }

    .menu-search input {
        width: 100%;
        box-sizing: border-box;
        padding: 7px 10px;
        border: 1px solid rgba(255, 255, 255, 0.14);
        border-radius: 0;
        outline: none;
        background: rgba(255, 255, 255, 0.06);
        color: #fff;
        font: inherit;
        font-size: 13px;
    }

    .menu-search input:focus {
        border-color: rgba(255, 255, 255, 0.34);
    }

    .menu-search input::placeholder {
        color: rgba(255, 255, 255, 0.4);
    }

    .menu-list {
        flex: 1;
        min-height: 0;
        overflow-y: auto;
    }

    ul {
        list-style: none;
        margin: 0;
        padding: 4px 0;
    }

    .selected-row {
        display: flex;
        flex: none;
        align-items: center;
        gap: 8px;
        width: 100%;
        height: 34px;
        padding: 0 12px;
        border: none;
        border-bottom: 1px solid rgba(255, 255, 255, 0.1);
        background: rgba(255, 255, 255, 0.04);
        color: #f87171;
        font-size: 12px;
        text-align: left;
        cursor: pointer;
    }

    .selected-row:hover,
    .selected-row:focus-visible {
        background: rgba(248, 113, 113, 0.12);
        outline: none;
    }

    .selected-title {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .selected-clear {
        flex: none;
        margin-left: auto;
        font-size: 16px;
        line-height: 1;
    }

    .task-item {
        display: flex;
        align-items: stretch;
    }

    .task-row {
        display: flex;
        flex: 1;
        align-items: center;
        justify-content: space-between;
        gap: 8px;
        min-width: 0;
        height: 34px;
        padding: 0 12px;
        border: none;
        border-left: 2px solid transparent;
        background: transparent;
        color: inherit;
        font-size: 13px;
        text-align: left;
        cursor: pointer;
    }

    .task-row:hover,
    .task-row:focus-visible {
        background: rgba(255, 255, 255, 0.08);
        outline: none;
    }

    .task-row.active {
        border-left-color: #eab308;
        background: rgba(234, 179, 8, 0.12);
    }

    .task-title {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .task-badge {
        flex: none;
        color: #eab308;
        font-size: 10px;
        letter-spacing: 0.08em;
        text-transform: uppercase;
    }

    .task-check {
        display: flex;
        flex: none;
        align-items: center;
        justify-content: center;
        width: 40px;
        padding: 0;
        border: none;
        border-left: 1px solid rgba(255, 255, 255, 0.08);
        background: transparent;
        color: rgba(255, 255, 255, 0.42);
        cursor: pointer;
    }

    .task-check:hover,
    .task-check:focus-visible {
        background: rgba(255, 255, 255, 0.08);
        color: #fff;
        outline: none;
    }

    .task-check.is-done {
        color: #eab308;
    }

    .empty {
        margin: 0;
        padding: 12px;
        color: rgba(255, 255, 255, 0.55);
        font-size: 12px;
        text-align: center;
    }
</style>
