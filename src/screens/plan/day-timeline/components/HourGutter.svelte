<script lang="ts">
    import { MINUTES_IN_HOUR, PX_PER_MIN } from '../constants';
    import { useCurrentTime } from '../hooks/useCurrentTime.svelte';
    import { formatHhMm } from '../../../../lib/time';

    const GUTTER_SIZE_MINUTES = 15;
    const time = useCurrentTime();
</script>

{#each Array.from({ length: 24 }) as _, h (h)}
    <span
        class="day-hour-label"
        style="top: {h * MINUTES_IN_HOUR * PX_PER_MIN}px; opacity: {Math.abs(h * MINUTES_IN_HOUR - time.value) < GUTTER_SIZE_MINUTES ? 0 : 1};"
    >
        {formatHhMm(h * MINUTES_IN_HOUR)}
    </span>
{/each}
<span class="day-now-label" style="top: {time.value * PX_PER_MIN}px;">{formatHhMm(time.value)}</span>
