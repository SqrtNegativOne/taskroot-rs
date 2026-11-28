<script lang="ts">
    import { HOURS_PER_DAY, MINUTES_IN_HOUR, PX_PER_MIN } from '../constants';
    import { useCurrentTime } from '../hooks/useCurrentTime.svelte';
    import { formatHhMm } from '../../../../lib/time';

    const GUTTER_SIZE_MINUTES = 15;

    let { startHour = 0 }: { startHour?: number } = $props();

    const time = useCurrentTime();
    let startMinute = $derived(startHour * MINUTES_IN_HOUR);
    let hours = $derived(
        Array.from({ length: HOURS_PER_DAY - startHour }, (_, i) => startHour + i),
    );
</script>

{#each hours as h (h)}
    {@const minute = h * MINUTES_IN_HOUR}
    {@const overlapsNow = time.value >= startMinute && Math.abs(minute - time.value) < GUTTER_SIZE_MINUTES}
    <span
        class="day-hour-label"
        style="top: {(h - startHour) * MINUTES_IN_HOUR * PX_PER_MIN}px; opacity: {overlapsNow ? 0 : 1};"
    >
        {formatHhMm(minute)}
    </span>
{/each}
{#if time.value >= startMinute}
    <span class="day-now-label" style="top: {(time.value - startMinute) * PX_PER_MIN}px;">{formatHhMm(time.value)}</span>
{/if}
