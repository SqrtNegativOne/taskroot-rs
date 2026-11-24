<script lang="ts">
    import { MINUTES_IN_HOUR, PX_PER_MIN } from '../constants';
</script>

{#each Array.from({ length: 24 }) as _, h (h)}
    <div
        class="day-hour"
        style="
            top: {h * MINUTES_IN_HOUR * PX_PER_MIN}px;
            height: {MINUTES_IN_HOUR * PX_PER_MIN}px;
        "
    >
        <div class="day-hour-line"></div>
        <div class="day-hour-half"></div>
    </div>
{/each}
