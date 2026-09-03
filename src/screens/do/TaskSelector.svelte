<script lang="ts">
    import type { AppTask } from '../../lib/domain';

    let {
        selectorOpen,
        onCloseSelector,
        tasks = [],
        onStartWithTask
    } = $props<{
        selectorOpen: boolean;
        onCloseSelector: () => void;
        tasks?: AppTask[];
        activeTask?: AppTask;
        onStartWithTask: (taskId: string) => void;
    }>();
</script>

{#if selectorOpen}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="task-selector-overlay" role="presentation" onclick={onCloseSelector}>
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div class="task-selector-modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()}>
            <h3>Select a task</h3>
            <ul class="task-list">
                {#each tasks as task (task.id)}
                    <li>
                        <button onclick={() => onStartWithTask(task.id)}>
                            {task.title}
                        </button>
                    </li>
                {/each}
            </ul>
            <button class="close-btn" onclick={onCloseSelector}>Close</button>
        </div>
    </div>
{/if}

<style>
    .task-selector-overlay {
        position: fixed;
        top: 0; left: 0; right: 0; bottom: 0;
        background: rgba(0, 0, 0, 0.5);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
    }
    .task-selector-modal {
        background: var(--bg-surface, #222);
        padding: 24px;
        border-radius: 8px;
        width: 400px;
        max-width: 90vw;
    }
    .task-list {
        list-style: none;
        padding: 0;
        margin: 16px 0;
    }
    .task-list li {
        margin-bottom: 8px;
    }
    .task-list button {
        width: 100%;
        text-align: left;
        padding: 8px;
        background: var(--bg-surface);
        border: none;
        border-radius: 4px;
        color: var(--fg);
        cursor: pointer;
    }
    .task-list button:hover {
        background: var(--bg-surface-hover);
    }
    .close-btn {
        background: none;
        border: 1px solid var(--border);
        color: var(--fg);
        padding: 8px 16px;
        border-radius: 4px;
        cursor: pointer;
    }
</style>
