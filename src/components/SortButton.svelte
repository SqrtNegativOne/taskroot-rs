<script lang="ts">
    type SortItem = any;

    let {
        sort = $bindable([]),
        sortOptions = [],
        align = "left",
    }: {
        sort?: SortItem[];
        sortOptions?: {id: string, label: string}[];
        align?: "left" | "right";
    } = $props();

    let sortMenuOpen = $state(false);
    let containerRef: HTMLDivElement | undefined = $state();

    $effect(() => {
        function onClick(e: MouseEvent) {
            if (containerRef && !containerRef.contains(e.target as Node)) {
                sortMenuOpen = false;
            }
        }
        document.addEventListener('pointerdown', onClick);
        return () => document.removeEventListener('pointerdown', onClick);
    });

    function addSort() {
        const id = sortOptions[0]?.id ?? 'priority';
        sort = [...(sort ?? []), { column: id, direction: 'asc' }];
    }

    function removeSort(index: number) {
        if (!sort) return;
        sort = sort.filter((_, i) => i !== index);
    }

    function updateSortCol(index: number, val: string) {
        if (!sort || index >= sort.length) return;
        sort[index].column = val;
        sort = [...sort];
    }

    function updateSortDir(index: number, val: string) {
        if (!sort || index >= sort.length) return;
        sort[index].direction = val;
        sort = [...sort];
    }

    let dragIndex = $state<number | null>(null);

    function handleDragStart(e: DragEvent, index: number) {
        dragIndex = index;
        if (e.dataTransfer) {
            e.dataTransfer.effectAllowed = 'move';
            e.dataTransfer.setData('text/plain', index.toString());
        }
    }

    function handleDragOver(e: DragEvent, index: number) {
        e.preventDefault();
        if (e.dataTransfer) {
            e.dataTransfer.dropEffect = 'move';
        }
    }

    function handleDrop(e: DragEvent, index: number) {
        e.preventDefault();
        if (sort && dragIndex !== null && dragIndex !== index) {
            const newSort = [...sort];
            const [item] = newSort.splice(dragIndex, 1);
            newSort.splice(index, 0, item);
            sort = newSort;
        }
        dragIndex = null;
    }
</script>

<div class="filter-sort-container" bind:this={containerRef}>
    {#if sortOptions.length > 0}
        <button
            class="menu-trigger-button"
            class:is-active={sortMenuOpen || (sort && sort.length > 0)}
            onclick={() => { sortMenuOpen = !sortMenuOpen; }}
            title="Sort"
        >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m3 16 4 4 4-4"/><path d="M7 20V4"/><path d="m21 8-4-4-4 4"/><path d="M17 4v16"/></svg>
            {#if sort && sort.length > 0}
                <span class="menu-trigger-badge">{sort.length}</span>
            {/if}
        </button>
    {/if}

    {#if sortMenuOpen && sortOptions.length > 0}
        <div class="shared-floating-menu align-{align}" style="min-width: 280px;">
            <div style="display: flex; flex-direction: column; gap: 8px;">
                {#if sort}
                    {#each sort as s, i}
                        <!-- svelte-ignore a11y_no_static_element_interactions -->
                        <div 
                            class="sort-row" 
                            draggable="true" 
                            ondragstart={(e) => handleDragStart(e, i)}
                            ondragover={(e) => handleDragOver(e, i)}
                            ondrop={(e) => handleDrop(e, i)}
                        >
                            <span class="drag-handle" title="Drag to reorder">░</span>
                            <select 
                                value={s.column} 
                                onchange={(e) => updateSortCol(i, e.currentTarget.value)}
                                style="flex: 1;"
                                class="form-select"
                            >
                                {#each sortOptions as o (o.id)}
                                    <option value={o.id}>{o.label}</option>
                                {/each}
                            </select>
                            <select
                                value={s.direction ?? 'asc'}
                                onchange={(e) => updateSortDir(i, e.currentTarget.value)}
                                class="form-select"
                            >
                                <option value="asc">Asc</option>
                                <option value="desc">Desc</option>
                            </select>
                            <button class="filter-remove-button" onclick={() => removeSort(i)} title="Remove Sort">
                                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
                            </button>
                        </div>
                    {/each}
                {/if}
                <button class="filter-add-button" onclick={addSort}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M5 12h14"/><path d="M12 5v14"/></svg>
                    Add Sort
                </button>
            </div>
        </div>
    {/if}
</div>

<style>
.filter-sort-container {
    position: relative;
    display: flex;
    gap: 8px;
    align-items: center;
}

.shared-floating-menu {
    position: absolute;
    top: calc(100% + 8px);
    z-index: 1000;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    background: var(--bg-surface);
    border-radius: 6px;
    border: 1px solid var(--border);
    box-shadow: 0 4px 12px rgba(0,0,0,0.2);
    animation: sharedFloatingMenuOpen 0.15s ease-out forwards;
    transform-origin: top center;
}

@keyframes sharedFloatingMenuOpen {
    from { opacity: 0; transform: scaleY(0.95); }
    to { opacity: 1; transform: scaleY(1); }
}

.shared-floating-menu.align-left {
    left: 0;
    right: auto;
}

.shared-floating-menu.align-right {
    left: auto;
    right: 0;
}

.menu-trigger-button {
    border: 1px solid var(--border);
    color: var(--fg);
    border-radius: 4px;
    padding: 4px 6px;
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
    background: transparent;
    transition: background 0.1s ease;
}

.menu-trigger-button.is-active,
.menu-trigger-button:hover {
    background: var(--bg-surface);
}

.menu-trigger-badge {
    font-size: 0.8em;
    font-weight: bold;
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

.filter-add-button {
    background: transparent;
    border: none;
    color: var(--fg);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 4px;
    align-self: flex-start;
    padding: 4px 4px;
    font-size: 0.9em;
    opacity: 0.8;
    transition: opacity 0.1s ease;
}

.filter-add-button:hover {
    opacity: 1;
}

.sort-row {
    display: flex;
    gap: 6px;
    align-items: center;
    background: var(--bg-surface);
    padding: 2px;
    border-radius: 4px;
}
.sort-row[draggable="true"] {
    cursor: grab;
}
.sort-row:active {
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

.sort-label {
    font-size: 0.9em;
    color: var(--fg);
    opacity: 0.8;
}

.form-select, .form-input {
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--fg);
    border-radius: 4px;
    padding: 4px;
    outline: none;
}
.form-select:focus, .form-input:focus {
    border-color: var(--accent);
}
</style>
