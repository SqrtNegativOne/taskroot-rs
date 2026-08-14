<script lang="ts">
    let { activeTask, onOpenSelector } = $props<{
        activeTask?: { title: string },
        onOpenSelector: () => void
    }>();
</script>

{#if activeTask}
    <button
        type="button"
        aria-label="Open task selector"
        class="active-task-btn"
        onclick={(e) => {
            e.stopPropagation();
            onOpenSelector();
        }}
        title="Click to change task"
    >
        <span class="active-task-label">Working on:</span>
        <span class="active-task-title">{activeTask.title}</span>
    </button>
{:else}
    <div class="no-active-task">No active task.</div>
{/if}
