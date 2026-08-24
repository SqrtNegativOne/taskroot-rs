<script lang="ts">
    import { MINUTES_IN_HOUR, PX_PER_MIN } from '../constants';
    import { useCurrentTime } from '../hooks/useCurrentTime.svelte.ts';

    let { showLabels = true }: { showLabels?: boolean } = $props();

    const time = useCurrentTime();

    function pad2(n: number) {
        return n.toString().padStart(2, '0');
    }
</script>

<div
    class="day-now"
    style="top: {time.value * PX_PER_MIN}px;"
>
    {#if showLabels}
        <span class="day-now-label">
            {pad2(Math.floor(time.value / MINUTES_IN_HOUR))}:{pad2(time.value % MINUTES_IN_HOUR)}
        </span>
    {/if}
    <div class="day-now-line" style={!showLabels ? "left: 8px;" : ""}></div>
</div>
