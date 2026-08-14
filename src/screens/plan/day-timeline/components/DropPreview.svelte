<script lang="ts">
    import { MINUTES_IN_HOUR, PX_PER_MIN, type DragStateTarget } from '../types';

    let { target }: { target: DragStateTarget } = $props();

    function hhmmShort(mins: number) {
        const h = Math.floor(mins / 60);
        const m = Math.floor(mins % 60);
        return `${h.toString()}:${m.toString().padStart(2, '0')}`;
    }
</script>

<div
    class="day-drop-preview"
    style="
        top: {(target.minute ?? 0) * PX_PER_MIN}px;
        height: {(target.duration ?? MINUTES_IN_HOUR) * PX_PER_MIN}px;
    "
>
    <span class="day-drop-preview-label">
        ▸ {hhmmShort(target.minute ?? 0)} – {hhmmShort((target.minute ?? 0) + (target.duration ?? MINUTES_IN_HOUR))}
    </span>
</div>
