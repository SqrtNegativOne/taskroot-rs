<script lang="ts">
    let { activeTask, label }: {
        activeTask?: { title: string };
        label?: string;
    } = $props();
</script>

{#if activeTask}
    <div class="active-task-label" title="Selected in the mini tracker">
        <span class="active-task-verb">Working on:</span>
        <span class="active-task-title">{activeTask.title}</span>
    </div>
{:else if label}
    <div class="active-task-label" title="Selected in the mini tracker">
        <span class="active-task-verb">Tracking:</span>
        <span class="active-task-title">{label}</span>
    </div>
{:else}
    <div class="no-active-task">No task selected. Pick one in the mini tracker.</div>
{/if}
