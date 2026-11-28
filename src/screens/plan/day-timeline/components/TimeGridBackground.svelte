<script lang="ts">
    import { HOURS_PER_DAY, MINUTES_IN_HOUR, PX_PER_MIN } from '../constants';

    let { startHour = 0 }: { startHour?: number } = $props();
    let hours = $derived(
        Array.from({ length: HOURS_PER_DAY - startHour }, (_, i) => startHour + i),
    );
</script>

{#each hours as h (h)}
    <div
        class="day-hour"
        style="
            top: {(h - startHour) * MINUTES_IN_HOUR * PX_PER_MIN}px;
            height: {MINUTES_IN_HOUR * PX_PER_MIN}px;
        "
    >
        <div class="day-hour-half"></div>
    </div>
{/each}
