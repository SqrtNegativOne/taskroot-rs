<script lang="ts" generics="F extends { column?: any | null, operator?: string | null, value?: any | null }">
    let {
        sort = $bindable('priority'),
        sortOptions = [
            { id: 'priority', label: 'Priority' },
            { id: 'due', label: 'Due Date' },
            { id: 'title', label: 'Title' },
            { id: 'id', label: 'Created' }
        ],
        align = "left",
    }: {
        sort?: string;
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
</script>

<div class="filter-sort-container" bind:this={containerRef}>
    {#if sortOptions.length > 0}
        <button class="menu-trigger-button {sortMenuOpen ? 'is-active' : ''}" onclick={() => { sortMenuOpen = !sortMenuOpen; }}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m3 16 4 4 4-4"/><path d="M7 20V4"/><path d="m21 8-4-4-4 4"/><path d="M17 4v16"/></svg>
            Sort
        </button>
    {/if}

    {#if sortMenuOpen && sortOptions.length > 0}
        <div class="shared-floating-menu align-{align}" style="min-width: 200px;">
            <div class="sort-row">
                <span class="sort-label">Sort by</span>
                <select 
                    value={sort} 
                    onchange={(e) => sort = e.currentTarget.value}
                    style="flex: 1;"
                    class="form-select"
                >
                    {#each sortOptions as o}
                        <option value={o.id}>{o.label}</option>
                    {/each}
                </select>
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

.filter-row {
    display: flex;
    gap: 6px;
    align-items: center;
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
