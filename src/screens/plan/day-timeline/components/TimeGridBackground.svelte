<script lang="ts">
    import { MINUTES_IN_HOUR, PX_PER_MIN } from '../types';
    import { useCurrentTime } from '../hooks/useCurrentTime.svelte';

    let { isToday, showLabels = true }: { isToday: boolean; showLabels?: boolean; } = $props();

    const GUTTER_SIZE_MINUTES = 15;
    const time = useCurrentTime();

    function pad2(n: number) {
        return n.toString().padStart(2, '0');
    }
</script>

<!-- eslint-disable-next-line @typescript-eslint/no-unused-vars -->
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
                {pad2(h)}:00
            </span>
        {/if}
        <div class="day-hour-line"></div>
        <div class="day-hour-half" style={!showLabels ? "left: 8px;" : ""}></div>
    </div>
{/each}
