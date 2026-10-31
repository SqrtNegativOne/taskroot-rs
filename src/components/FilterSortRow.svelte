<script lang="ts">
    import type { Snippet } from 'svelte';

    let {
        draggable = false,
        ondragstart,
        ondragover,
        ondrop,
        onremove,
        removeTitle = 'Remove',
        children,
    }: {
        draggable?: boolean;
        ondragstart?: (event: DragEvent) => void;
        ondragover?: (event: DragEvent) => void;
        ondrop?: (event: DragEvent) => void;
        onremove: () => void;
        removeTitle?: string;
        children?: Snippet;
    } = $props();
</script>

<div
    class="filter-sort-row"
    class:is-draggable={draggable}
    draggable={draggable ? true : undefined}
    {ondragstart}
    {ondragover}
    {ondrop}
>
    {#if draggable}
        <span class="drag-handle" title="Drag to reorder">░</span>
    {/if}
    {@render children?.()}
    <button class="filter-remove-button" onclick={onremove} title={removeTitle} aria-label={removeTitle}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
    </button>
</div>

<style>
.filter-sort-row {
    display: flex;
    gap: 6px;
    align-items: center;
}

.filter-sort-row.is-draggable {
    background: var(--bg-surface);
    padding: 2px;
    border-radius: 4px;
    cursor: grab;
}

.filter-sort-row.is-draggable:active {
    cursor: grabbing;
}

.drag-handle {
    cursor: grab;
    color: var(--fg);
    opacity: 0.4;
    user-select: none;
}

.drag-handle:active {
    cursor: grabbing;
}

.filter-remove-button {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 4px;
    color: var(--fg);
    opacity: 0.6;
    transition: opacity 0.1s ease;
}

.filter-remove-button:hover {
    opacity: 1;
}

.filter-sort-row :global(.form-select),
.filter-sort-row :global(.form-input) {
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--fg);
    border-radius: 4px;
    padding: 4px;
    outline: none;
}

.filter-sort-row :global(.form-select:focus),
.filter-sort-row :global(.form-input:focus) {
    border-color: var(--accent);
}
</style>
