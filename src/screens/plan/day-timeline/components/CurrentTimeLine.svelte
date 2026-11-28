<script lang="ts">
    import { PX_PER_MIN } from '../constants';
    import { useCurrentTime } from '../hooks/useCurrentTime.svelte';

    let { startMinute = 0 }: { startMinute?: number } = $props();

    const time = useCurrentTime();
</script>

{#if time.value >= startMinute}
    <div class="day-now" style="top: {(time.value - startMinute) * PX_PER_MIN}px;">
        <div class="day-now-line"></div>
    </div>
{/if}
