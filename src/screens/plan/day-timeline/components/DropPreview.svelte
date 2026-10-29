<script lang="ts">
    import { MINUTES_IN_HOUR, PX_PER_MIN } from '../constants';
    import type { DragStateTarget } from '../types';
    import { formatHhMm } from '../../../../lib/time';

    let { target }: { target: DragStateTarget } = $props();
</script>

<div
    class="day-drop-preview"
    style="
        top: {(target.minute ?? 0) * PX_PER_MIN}px;
        height: {(target.duration ?? MINUTES_IN_HOUR) * PX_PER_MIN}px;
    "
>
    <span class="day-drop-preview-label">
        ▸ {formatHhMm(target.minute ?? 0)} – {formatHhMm((target.minute ?? 0) + (target.duration ?? MINUTES_IN_HOUR))}
    </span>
</div>
