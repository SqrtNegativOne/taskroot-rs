<script lang="ts">
    import { PX_PER_MIN } from '../constants';
    import { formatHhMm } from '../../../../lib/time';

    let { preview }: { preview: { start: number; end: number } } = $props();
</script>

<div
    class="day-event ev-plan is-compact"
    style="
        top: {preview.start * PX_PER_MIN}px;
        height: {(preview.end - preview.start) * PX_PER_MIN}px;
        opacity: 0.5;
        pointer-events: none;
        z-index: 10;
    "
>
    <div class="day-event-inner">
        <div class="day-event-time">
            {formatHhMm(preview.start)} – {formatHhMm(preview.end)}
        </div>
        <div class="day-event-title">New Event...</div>
    </div>
</div>
