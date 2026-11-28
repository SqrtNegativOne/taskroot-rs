<script lang="ts">
    import BentoCard from './BentoCard.svelte';
    import { formatDuration } from './metrics';
    import type { DailyActivityPoint } from './types';

    interface Props {
        days: readonly DailyActivityPoint[];
    }

    let { days }: Props = $props();

    let activeIndex = $state<number | null>(null);

    const maxMinutes = $derived(
        Math.max(1, ...days.map((day) => day.trackedMinutes)),
    );
    const maxDone = $derived(
        Math.max(1, ...days.map((day) => day.completedCount)),
    );
    const hasAnyActivity = $derived(
        days.some((day) => day.trackedMinutes > 0 || day.completedCount > 0),
    );
    const totalWindowMinutes = $derived(
        days.reduce((sum, day) => sum + day.trackedMinutes, 0),
    );
    const totalWindowDone = $derived(
        days.reduce((sum, day) => sum + day.completedCount, 0),
    );
    const inspectedDay = $derived(
        activeIndex !== null ? days[activeIndex] ?? null : null,
    );

    function minuteBarHeight(minutes: number): string {
        if (minutes <= 0) return '0%';
        return `${Math.max(8, Math.round((minutes / maxMinutes) * 100))}%`;
    }

    function doneBarHeight(count: number): string {
        if (count <= 0) return '0%';
        return `${Math.max(8, Math.round((count / maxDone) * 100))}%`;
    }
</script>

<BentoCard title="14-day activity" span="col-8" testId="card-daily-activity">
    {#snippet badge()}
        <span class="chart-legend">
            <span class="legend-item">
                <span class="legend-swatch is-focus"></span>
                {formatDuration(totalWindowMinutes * 60)} focus
            </span>
            <span class="legend-item">
                <span class="legend-swatch is-done"></span>
                {totalWindowDone} done
            </span>
        </span>
    {/snippet}

    <div class="activity-chart-wrap">
        <div class="activity-bars" role="list" aria-label="Last 14 days activity">
            {#each days as day, index (day.date)}
                <button
                    type="button"
                    class="activity-day"
                    class:is-active={activeIndex === index}
                    title={`${day.weekday}, ${day.label}: ${formatDuration(day.trackedMinutes * 60)} focus, ${day.completedCount} completed`}
                    onmouseenter={() => { activeIndex = index; }}
                    onmouseleave={() => { activeIndex = null; }}
                    onfocus={() => { activeIndex = index; }}
                    onblur={() => { activeIndex = null; }}
                >
                    <span class="activity-bar-pair">
                        <span class="activity-track">
                            <span
                                class="activity-fill is-focus"
                                style:height={minuteBarHeight(day.trackedMinutes)}
                            ></span>
                        </span>
                        <span class="activity-track">
                            <span
                                class="activity-fill is-done"
                                style:height={doneBarHeight(day.completedCount)}
                            ></span>
                        </span>
                    </span>
                    <span class="activity-day-num">{day.shortLabel}</span>
                    <span class="activity-day-wk">{day.weekday.slice(0, 1)}</span>
                </button>
            {/each}
        </div>

        <div class="activity-footer">
            {#if inspectedDay}
                <span class="activity-readout">
                    <strong>{inspectedDay.weekday} {inspectedDay.label}</strong>
                    · {formatDuration(inspectedDay.trackedMinutes * 60)} tracked · {inspectedDay.completedCount} completed
                </span>
            {:else if !hasAnyActivity}
                <span class="activity-hint">No completions or focus sessions logged in the last 14 days.</span>
            {:else}
                <span class="activity-hint">Hover or focus a day bar for exact focus minutes and completed tasks.</span>
            {/if}
        </div>
    </div>
</BentoCard>
