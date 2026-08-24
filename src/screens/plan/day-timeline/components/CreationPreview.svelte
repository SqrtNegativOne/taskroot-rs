<script lang="ts">
    import { PIXELS_PER_HOUR, PX_PER_MIN } from '../constants';

    let { preview }: { preview: { start: number; end: number } } = $props();

    function hhmmShort(mins: number) {
        const h = Math.floor(mins / 60);
        const m = Math.floor(mins % 60);
        return `${h.toString()}:${m.toString().padStart(2, '0')}`;
    }
</script>

<div
    class="day-event ev-plan is-compact"
    style="
        top: {preview.start * PX_PER_MIN}px;
        height: {(preview.end - preview.start) * PX_PER_MIN}px;
        left: {PIXELS_PER_HOUR}px;
        width: calc(100% - {PIXELS_PER_HOUR + 2}px);
        opacity: 0.5;
        pointer-events: none;
        z-index: 10;
    "
>
    <div class="day-event-inner">
        <div class="day-event-time">
            {hhmmShort(preview.start)} – {hhmmShort(preview.end)}
        </div>
        <div class="day-event-title">New Event...</div>
    </div>
</div>
