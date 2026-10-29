<script lang="ts">
    import { PX_PER_MIN } from '../constants';
    import { useCurrentTime } from '../hooks/useCurrentTime.svelte.ts';
    import { formatHhMm } from '../../../../lib/time';

    let { showLabels = true }: { showLabels?: boolean } = $props();

    const time = useCurrentTime();
</script>

<div
    class="day-now"
    style="top: {time.value * PX_PER_MIN}px;"
>
    {#if showLabels}
        <span class="day-now-label">
            {formatHhMm(time.value)}
        </span>
    {/if}
    <div class="day-now-line" style={!showLabels ? "left: 8px;" : ""}></div>
</div>
