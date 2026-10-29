<script lang="ts">
    import { MINUTES_IN_HOUR, PX_PER_MIN } from '../constants';
    import { useCurrentTime } from '../hooks/useCurrentTime.svelte';
    import { formatHhMm } from '../../../../lib/time';

    let { isToday, showLabels = true }: { isToday: boolean; showLabels?: boolean; } = $props();

    const GUTTER_SIZE_MINUTES = 15;
    const time = useCurrentTime();
</script>

{#each Array.from({ length: 24 }) as _, h (h)}
    <div
        class="day-hour"
        style="
            top: {h * MINUTES_IN_HOUR * PX_PER_MIN}px;
            height: {MINUTES_IN_HOUR * PX_PER_MIN}px;
        "
    >
        {#if showLabels}
            <span
                class="day-hour-label"
                style="opacity: {isToday && Math.abs(h * MINUTES_IN_HOUR - time.value) < GUTTER_SIZE_MINUTES ? 0 : 1}"
            >
                {formatHhMm(h * MINUTES_IN_HOUR)}
            </span>
        {/if}
        <div class="day-hour-line"></div>
        <div class="day-hour-half" style={!showLabels ? "left: 8px;" : ""}></div>
    </div>
{/each}
