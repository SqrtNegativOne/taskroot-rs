<script lang="ts">
    import BentoCard from './BentoCard.svelte';
    import { formatDuration } from './metrics';
    import type { TaskTimeStat } from './types';

    interface Props {
        tasks: readonly TaskTimeStat[];
    }

    let { tasks }: Props = $props();

    const maxTracked = $derived(
        Math.max(1, ...tasks.map((task) => task.trackedSeconds)),
    );

    function barWidth(seconds: number): string {
        return `${Math.max(4, Math.round((seconds / maxTracked) * 100))}%`;
    }
</script>

<BentoCard title="top tasks by time" span="col-6" testId="card-top-tasks">
    {#snippet badge()}
        <span class="badge-muted">{tasks.length} tracked</span>
    {/snippet}

    {#if tasks.length === 0}
        <div class="empty-state">No task focus time recorded yet.</div>
    {:else}
        <div class="rank-list">
            {#each tasks as task (task.id)}
                <div class="rank-row">
                    <div class="rank-row-head">
                        <span class="rank-title" class:is-done={task.status === 'done'}>
                            {task.title}
                        </span>
                        <span class="rank-meta">
                            {#if task.estMinutes !== null}
                                <span class="est-badge">est {formatDuration(task.estMinutes * 60)}</span>
                            {/if}
                            <span class="rank-val">{formatDuration(task.trackedSeconds)}</span>
                        </span>
                    </div>
                    <div class="bar-track">
                        <div class="bar-fill is-accent" style:width={barWidth(task.trackedSeconds)}></div>
                    </div>
                </div>
            {/each}
        </div>
    {/if}
</BentoCard>
