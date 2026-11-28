<script lang="ts">
    import type { AppTask } from '../../lib/domain';
    import { store } from '../../lib/store.svelte';
    import { formatHhMm, minutesSinceMidnight } from '../../lib/time';

    interface Props {
        tasks: readonly AppTask[];
        completedTasks: readonly AppTask[];
        wrapYmd: string;
        tomorrowYmd: string;
    }

    let { tasks, completedTasks, wrapYmd, tomorrowYmd }: Props = $props();

    let unfinishedNextUp = $derived(
        tasks.filter(
            (task) =>
                task.status === 'next-up'
                && (!task.start || task.start.slice(0, 10) <= wrapYmd),
        ),
    );

    function formatDoneTime(completedAt: string | null | undefined): string {
        if (!completedAt) return '';
        const date = new Date(completedAt);
        if (Number.isNaN(date.getTime())) return '';
        return formatHhMm(minutesSinceMidnight(date));
    }

    function markDone(task: AppTask): void {
        void store.mutate('Failed to complete task', () => store.toggleTask(task.id));
    }

    function pushToTomorrow(task: AppTask): void {
        void store.mutate('Failed to push task to tomorrow', () =>
            store.updateTask(task.id, [{ field: 'start', value: tomorrowYmd }]),
        );
    }

    function moveToBacklog(task: AppTask): void {
        void store.mutate('Failed to move task to backlog', () =>
            store.updateTask(task.id, [{ field: 'status', value: 'todo' }]),
        );
    }
</script>

<section class="wrap-card" aria-label="Today's Next Up Review">
    <header class="wrap-card-hd">
        <h3 class="wrap-card-title">What you planned</h3>
        <span class="wrap-card-subtitle">next-up review</span>
    </header>

    <div class="wrap-subsection">
        <div class="wrap-sub-hd">
            <span>Didn't finish</span>
            <span>{unfinishedNextUp.length}</span>
        </div>
        {#if unfinishedNextUp.length === 0}
            <p class="wrap-empty-note">All planned next-up tasks are cleared.</p>
        {:else}
            <div class="wrap-task-list">
                {#each unfinishedNextUp as task (task.id)}
                    <div class="wrap-task-row">
                        <div class="wrap-task-info">
                            <span class="wrap-task-title">{task.title}</span>
                            {#if task.est}
                                <span class="wrap-task-meta">{task.est}m</span>
                            {/if}
                        </div>
                        <div class="wrap-task-actions">
                            <button
                                type="button"
                                class="wrap-btn is-success"
                                onclick={() => markDone(task)}
                            >
                                Done
                            </button>
                            <button
                                type="button"
                                class="wrap-btn is-accent"
                                onclick={() => pushToTomorrow(task)}
                            >
                                Tomorrow
                            </button>
                            <button
                                type="button"
                                class="wrap-btn"
                                onclick={() => moveToBacklog(task)}
                            >
                                Backlog
                            </button>
                        </div>
                    </div>
                {/each}
            </div>
        {/if}
    </div>

    <div class="wrap-subsection">
        <div class="wrap-sub-hd">
            <span>Finished today</span>
            <span>{completedTasks.length}</span>
        </div>
        {#if completedTasks.length === 0}
            <p class="wrap-empty-note">No tasks completed on this day yet.</p>
        {:else}
            <div class="wrap-task-list">
                {#each completedTasks as task (task.id)}
                    <div class="wrap-task-row">
                        <div class="wrap-task-info">
                            <span class="wrap-check" aria-hidden="true">✓</span>
                            <span class="wrap-task-title">{task.title}</span>
                        </div>
                        {#if task.completedAt}
                            <span class="wrap-task-meta">{formatDoneTime(task.completedAt)}</span>
                        {/if}
                    </div>
                {/each}
            </div>
        {/if}
    </div>
</section>
