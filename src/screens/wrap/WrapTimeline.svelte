<script lang="ts">
    import { HOURS_PER_DAY, MINUTES_IN_HOUR, PX_PER_MIN } from '../plan/day-timeline/constants';
    import { formatHhMm } from '../../lib/time';
    import { formatWrapDateLabel } from './wrapWindow';
    import { formatWrapDuration, type WrapDaySummary, type WrapTimelineItem } from './wrapData';

    interface Props {
        summary: WrapDaySummary;
        isYesterday: boolean;
    }

    let { summary, isYesterday }: Props = $props();

    let scrollRef = $state<HTMLDivElement | null>(null);
    let didInitialScroll = false;

    let startMins = $derived(summary.startHour * MINUTES_IN_HOUR);
    let hours = $derived(
        Array.from({ length: HOURS_PER_DAY - summary.startHour }, (_, i) => summary.startHour + i),
    );
    let gridHeightPx = $derived(hours.length * MINUTES_IN_HOUR * PX_PER_MIN);

    function topPx(mins: number): number {
        return Math.max(0, (mins - startMins) * PX_PER_MIN);
    }

    function heightPx(item: WrapTimelineItem): number {
        return Math.max(14, (item.layoutEndMins - item.layoutStartMins) * PX_PER_MIN);
    }

    function isCompactItem(item: WrapTimelineItem): boolean {
        return heightPx(item) < 38;
    }

    $effect(() => {
        if (didInitialScroll || !scrollRef || summary.items.length === 0) return;
        const firstTop = topPx(summary.items[0]!.layoutStartMins);
        scrollRef.scrollTop = Math.max(0, firstTop - 40);
        didInitialScroll = true;
    });
</script>

<section class="wrap-timeline-pane" aria-label="Day timeline">
    <header class="wrap-timeline-hd">
        <div class="wrap-date-group">
            <h2 class="wrap-date-title">{formatWrapDateLabel(summary.wrapDate)}</h2>
            {#if isYesterday}
                <span class="wrap-yesterday-pill">(Yesterday)</span>
            {/if}
        </div>
        <div class="wrap-summary-pills">
            <span class="wrap-pill is-done">
                ✓ {summary.completedCount} {summary.completedCount === 1 ? 'task' : 'tasks'} done
            </span>
            <span class="wrap-pill">
                {formatWrapDuration(summary.totalTrackedMs)} focused
            </span>
        </div>
    </header>

    <div class="wrap-timeline-scroll" bind:this={scrollRef}>
        {#if summary.items.length === 0}
            <p class="wrap-timeline-empty">
                No completed tasks or tracked focus sessions recorded for this day yet.
            </p>
        {/if}

        <div class="wrap-timeline-grid-wrap" style="height: {gridHeightPx}px;">
            <div class="wrap-gutter" style="height: {gridHeightPx}px;">
                {#each hours as h (h)}
                    <span
                        class="wrap-hour-label"
                        style="top: {(h - summary.startHour) * MINUTES_IN_HOUR * PX_PER_MIN}px;"
                    >
                        {formatHhMm(h * MINUTES_IN_HOUR)}
                    </span>
                {/each}
            </div>

            <div class="wrap-day-col" style="height: {gridHeightPx}px;">
                {#each hours as h (h)}
                    <div
                        class="wrap-hour-line"
                        style="top: {(h - summary.startHour) * MINUTES_IN_HOUR * PX_PER_MIN}px; height: {MINUTES_IN_HOUR * PX_PER_MIN}px;"
                    >
                        <div class="wrap-half-line"></div>
                    </div>
                {/each}

                {#each summary.items as item (item.id)}
                    {#if item.kind === 'completion-pin'}
                        <div
                            class="wrap-pin"
                            data-wrap-item={item.kind}
                            style="top: {topPx(item.completedAtMins ?? item.layoutStartMins)}px; --lane: {item.lane}; --lanes: {item.lanes};"
                        >
                            <span class="wrap-pin-badge">
                                <span class="wrap-pin-time">
                                    ✓ {formatHhMm(item.completedAtMins ?? item.startMins)}
                                </span>
                                <span class="wrap-pin-title">{item.title}</span>
                            </span>
                        </div>
                    {:else}
                        <article
                            class="wrap-block"
                            class:is-work={item.kind === 'work'}
                            class:is-completed={item.kind === 'completed'}
                            class:is-done={item.isDone}
                            class:is-compact={isCompactItem(item)}
                            data-wrap-item={item.kind}
                            style="top: {topPx(item.layoutStartMins)}px; height: {heightPx(item)}px; --lane: {item.lane}; --lanes: {item.lanes};"
                        >
                            <div class="wrap-block-head">
                                {#if item.isDone}
                                    <span class="wrap-check" aria-label="Done">✓</span>
                                {/if}
                                <span class="wrap-block-title">{item.title}</span>
                            </div>
                            <div class="wrap-block-meta">
                                {#if item.kind === 'work'}
                                    <span>{formatHhMm(item.startMins)}–{formatHhMm(item.endMins)}</span>
                                    {#if item.durationMs !== undefined}
                                        <span class="wrap-dur-badge">{formatWrapDuration(item.durationMs)}</span>
                                    {/if}
                                {:else}
                                    <span>Done {formatHhMm(item.completedAtMins ?? item.endMins)}</span>
                                {/if}
                            </div>
                        </article>
                    {/if}
                {/each}
            </div>
        </div>
    </div>
</section>
