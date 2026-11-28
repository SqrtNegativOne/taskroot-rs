<script lang="ts">
    import { MINUTES_IN_HOUR, PX_PER_MIN } from '../constants';
    import type { DragStateTarget } from '../types';
    import { formatHhMm } from '../../../../lib/time';

    let {
        target,
        startMinute = 0,
    }: {
        target: DragStateTarget;
        startMinute?: number;
    } = $props();

    const minute = $derived(target.minute ?? startMinute);
    const duration = $derived(target.duration ?? MINUTES_IN_HOUR);
</script>

<div
    class="day-drop-preview"
    style="
        top: {(minute - startMinute) * PX_PER_MIN}px;
        height: {duration * PX_PER_MIN}px;
    "
>
    <span class="day-drop-preview-label">
        ▸ {formatHhMm(minute)} – {formatHhMm(minute + duration)}
    </span>
</div>
