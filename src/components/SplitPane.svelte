<script lang="ts">
    import { untrack } from 'svelte';
    import type { Snippet } from 'svelte';

    type Direction = 'horizontal' | 'vertical';

    let {
        direction = 'vertical',
        minSize = 100,
        snapThreshold = 50,
        defaultSize = 300,
        pane1,
        pane2
    }: {
        direction?: Direction;
        minSize?: number;
        snapThreshold?: number;
        defaultSize?: number;
        pane1: Snippet;
        pane2: Snippet;
    } = $props();

    let size = $state(untrack(() => defaultSize));
    let isDragging = $state(false);
    let containerRef = $state<HTMLDivElement | null>(null);

    let isHoriz = $derived(direction === 'horizontal');

    function onPointerDown(e: PointerEvent) {
        e.preventDefault();
        isDragging = true;
        if (e.target instanceof Element) {
            e.target.setPointerCapture(e.pointerId);
        }
    }

    function onPointerMove(e: PointerEvent) {
        if (!isDragging || !containerRef) return;
        const rect = containerRef.getBoundingClientRect();
        
        let newSize = isHoriz ? e.clientX - rect.left : e.clientY - rect.top;

        if (newSize < snapThreshold) newSize = 0;

        const maxSize = isHoriz ? rect.width : rect.height;
        if (newSize > maxSize - snapThreshold) newSize = maxSize;

        size = newSize;
    }

    function onPointerUp(e: PointerEvent) {
        isDragging = false;
        if (e.target instanceof Element) {
            e.target.releasePointerCapture(e.pointerId);
        }
    }
</script>

<div
    bind:this={containerRef}
    class="split-pane-container"
    style="
        flex-direction: {isHoriz ? 'row' : 'column'};
        pointer-events: {isDragging ? 'none' : 'auto'};
    "
>
    <div
        class="first-child"
        style="
            {isHoriz ? 'width' : 'height'}: {size}px;
            {isHoriz ? 'min-width' : 'min-height'}: {size > 0 ? minSize : 0}px;
            display: {size === 0 ? 'none' : 'flex'};
        "
    >
        {@render pane1()}
    </div>

    <div
        role="separator"
        tabindex="-1"
        onpointerdown={onPointerDown}
        onpointermove={onPointerMove}
        onpointerup={onPointerUp}
        class="split-pane-divider"
        class:dragging={isDragging}
        style="
            {isHoriz ? 'width' : 'height'}: 6px;
            {isHoriz ? 'margin-left' : 'margin-top'}: -3px;
            {isHoriz ? 'margin-right' : 'margin-bottom'}: -3px;
            cursor: {isHoriz ? 'col-resize' : 'row-resize'};
        "
    ></div>

    <div class="second-child">
        {@render pane2()}
    </div>
</div>

<style>
    .split-pane-container {
        display: flex;
        width: 100%;
        height: 100%;
        overflow: hidden;
    }

    .first-child {
        flex-direction: column;
        overflow: hidden;
        flex-shrink: 0;
    }

    .split-pane-divider {
        flex-shrink: 0;
        z-index: 10;
        position: relative;
        pointer-events: auto;
        background-color: transparent;
        transition: background-color 0.15s ease;
    }

    .split-pane-divider:hover,
    .split-pane-divider.dragging {
        background-color: var(--accent, #3b82f6);
    }

    .second-child {
        flex: 1;
        min-width: 0;
        min-height: 0;
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }
</style>
