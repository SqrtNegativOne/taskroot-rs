<script lang="ts">
    import BentoCard from './BentoCard.svelte';
    import { formatDuration } from './metrics';
    import type { HourlyActivityPoint } from './types';

    interface Props {
        buckets: readonly HourlyActivityPoint[];
        peakHour: number | null;
        peakHourLabel: string;
    }

    let { buckets, peakHour, peakHourLabel }: Props = $props();

    let hoveredHour = $state<number | null>(null);

    const maxScore = $derived(
        Math.max(1, ...buckets.map((bucket) => bucket.score)),
    );
    const hasActivity = $derived(peakHour !== null);
    const activeBucket = $derived(
        hoveredHour !== null ? buckets[hoveredHour] ?? null : null,
    );

    function sparkHeight(score: number): string {
        if (score <= 0) return '4%';
        return `${Math.max(10, Math.round((score / maxScore) * 100))}%`;
    }
</script>

<BentoCard title="peak hours" span="col-4" testId="card-peak-hours">
    {#snippet badge()}
        <span class="peak-badge" class:is-empty={!hasActivity}>{peakHourLabel}</span>
    {/snippet}

    <div class="peak-wrap">
        <div class="peak-sparkbars" role="list" aria-label="Hourly productivity distribution">
            {#each buckets as bucket (bucket.hour)}
                <button
                    type="button"
                    class="peak-bar-btn"
                    class:is-peak={peakHour === bucket.hour}
                    class:is-hovered={hoveredHour === bucket.hour}
                    title={`${bucket.label}:00 — ${formatDuration(bucket.trackedMinutes * 60)} focus, ${bucket.completedCount} done`}
                    onmouseenter={() => { hoveredHour = bucket.hour; }}
                    onmouseleave={() => { hoveredHour = null; }}
                    onfocus={() => { hoveredHour = bucket.hour; }}
                    onblur={() => { hoveredHour = null; }}
                >
                    <span
                        class="peak-bar"
                        class:is-empty={bucket.score === 0}
                        style:height={sparkHeight(bucket.score)}
                    ></span>
                </button>
            {/each}
        </div>

        <div class="peak-axis">
            <span>00</span>
            <span>06</span>
            <span>12</span>
            <span>18</span>
            <span>23</span>
        </div>

        <div class="peak-readout">
            {#if activeBucket}
                <span>
                    <strong>{activeBucket.label}:00</strong> · {formatDuration(activeBucket.trackedMinutes * 60)} · {activeBucket.completedCount} done
                </span>
            {:else if hasActivity}
                <span class="activity-hint">Most productive window: {peakHourLabel}</span>
            {:else}
                <span class="activity-hint">Complete tasks or track time to surface your peak hours.</span>
            {/if}
        </div>
    </div>
</BentoCard>
