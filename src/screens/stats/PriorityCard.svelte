<script lang="ts">
    import BentoCard from './BentoCard.svelte';
    import type { PriorityStat } from './types';

    interface Props {
        priorities: readonly PriorityStat[];
        totalTasks: number;
    }

    let { priorities, totalTasks }: Props = $props();

    const maxTotal = $derived(
        Math.max(1, ...priorities.map((row) => row.total)),
    );

    function barWidth(total: number): string {
        if (total <= 0) return '0%';
        return `${Math.max(4, Math.round((total / maxTotal) * 100))}%`;
    }
</script>

<BentoCard title="priority distribution" span="col-6" testId="card-priority">
    {#snippet badge()}
        <span class="badge-muted">{totalTasks} total</span>
    {/snippet}

    <div class="rank-list">
        {#each priorities as row (row.priority)}
            <div class="rank-row">
                <div class="rank-row-head">
                    <span class="priority-label">
                        <span class="priority-dot" style:background={`var(${row.cssVar})`}></span>
                        {row.label}
                    </span>
                    <span class="rank-meta">
                        <span class="rank-sub">{row.done}/{row.total} done ({row.completionPct}%)</span>
                        <span class="rank-val">{row.total}</span>
                    </span>
                </div>
                <div class="bar-track">
                    <div
                        class="bar-fill"
                        style:width={barWidth(row.total)}
                        style:background={`var(${row.cssVar})`}
                    ></div>
                </div>
            </div>
        {/each}
    </div>
</BentoCard>
