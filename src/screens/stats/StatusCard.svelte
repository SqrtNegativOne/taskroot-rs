<script lang="ts">
    import BentoCard from './BentoCard.svelte';

    interface Props {
        totalTasks: number;
        doneTasks: number;
        nextUpTasks: number;
        openTasks: number;
        cancelledTasks: number;
    }

    let { totalTasks, doneTasks, nextUpTasks, openTasks, cancelledTasks }: Props = $props();

    const segments = $derived([
        { key: 'done', label: 'Done', count: doneTasks, color: 'var(--success)' },
        { key: 'next-up', label: 'Next Up', count: nextUpTasks, color: 'var(--accent)' },
        { key: 'todo', label: 'Todo', count: openTasks, color: 'var(--fg-dim)' },
        { key: 'cancelled', label: 'Cancelled', count: cancelledTasks, color: 'var(--fg-dimmest)' },
    ]);

    function pct(count: number): number {
        if (totalTasks <= 0) return 0;
        return Math.round((count / totalTasks) * 100);
    }

    function segmentWidth(count: number): string {
        if (totalTasks <= 0 || count <= 0) return '0%';
        return `${(count / totalTasks) * 100}%`;
    }
</script>

<BentoCard title="status overview" span="col-6" testId="card-status">
    {#snippet badge()}
        <span class="badge-muted">{totalTasks} tasks</span>
    {/snippet}

    {#if totalTasks === 0}
        <div class="empty-state">No tasks yet.</div>
    {:else}
        <div class="stacked-wrap">
            <div class="stacked-bar" aria-label="Task status distribution">
                {#each segments as seg (seg.key)}
                    {#if seg.count > 0}
                        <div
                            class="stacked-seg"
                            style:width={segmentWidth(seg.count)}
                            style:background={seg.color}
                            title={`${seg.label}: ${seg.count} (${pct(seg.count)}%)`}
                        ></div>
                    {/if}
                {/each}
            </div>

            <div class="status-grid">
                {#each segments as seg (seg.key)}
                    <div class="status-item">
                        <span class="status-label">
                            <span class="legend-swatch" style:background={seg.color}></span>
                            {seg.label}
                        </span>
                        <span class="status-val">
                            {seg.count}
                            <span class="status-pct">{pct(seg.count)}%</span>
                        </span>
                    </div>
                {/each}
            </div>
        </div>
    {/if}
</BentoCard>
