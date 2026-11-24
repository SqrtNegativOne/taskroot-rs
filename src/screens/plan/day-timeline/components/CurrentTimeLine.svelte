<script lang="ts">
    import { PX_PER_MIN } from '../constants';
    import { useCurrentTime } from '../hooks/useCurrentTime.svelte.ts';

    const time = useCurrentTime();
</script>

<div class="day-now" style="top: {time.value * PX_PER_MIN}px;">
    <div class="day-now-line"></div>
</div>
