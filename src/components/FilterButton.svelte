<script lang="ts" generics="F extends { column?: any | null, operator?: string | null, value?: any | null }">
    let {
        filters = $bindable([]),
        columns = [
            { id: 'status', label: 'Status' },
            { id: 'priority', label: 'Priority' },
            { id: 'tag', label: 'Tag' }
        ],
        getValuesForColumn = (col: string) => {
            if (col === 'status') return ['todo', 'doing', 'done', 'nextup'];
            if (col === 'priority') return ['0', '1', '2', '3', '4'];
            return [];
        },
        align = "left",
    }: {
        filters: F[];
        columns?: {id: string, label: string}[];
        getValuesForColumn?: (col: string) => string[];
        align?: "left" | "right";
    } = $props();

    let filterMenuOpen = $state(false);
    let containerRef: HTMLDivElement | undefined = $state();

    $effect(() => {
        function onClick(e: MouseEvent) {
            if (containerRef && !containerRef.contains(e.target as Node)) {
                filterMenuOpen = false;
            }
        }
        document.addEventListener('pointerdown', onClick);
        return () => document.removeEventListener('pointerdown', onClick);
    });

    function addFilter() {
        const id = columns[0]?.id || 'status';
        filters = [...filters, { column: id, operator: 'is', value: [] } as unknown as F];
    }

    function removeFilter(index: number) {
        filters = filters.filter((_, i) => i !== index);
    }

    function updateFilter(index: number, updates: Partial<F>) {
        const newFilters = [...filters];
        newFilters[index] = { ...newFilters[index], ...(updates as any) };
        filters = newFilters;
    }
</script>

<div class="filter-sort-container" bind:this={containerRef}>
    <button class="menu-trigger-button {filterMenuOpen || filters.length > 0 ? 'is-active' : ''}" onclick={() => { filterMenuOpen = !filterMenuOpen; }}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"/></svg>
        Filter
        {#if filters.length > 0}
            <span class="menu-trigger-badge">{filters.length}</span>
        {/if}
    </button>

    {#if filterMenuOpen}
        <div class="shared-floating-menu align-{align}" style="min-width: 320px;">
            {#each filters as f, i}
                <div class="filter-row">
                    <select 
                        value={f.column} 
                        onchange={(e) => {
                            const val = e.currentTarget.value;
                            if (val === 'status' || val === 'priority' || val === 'tag') {
                                updateFilter(i, { column: val, value: [] } as any);
                            }
                        }}
                        style="flex: 1;"
                        class="form-select"
                    >
                        {#each columns as c}
                            <option value={c.id}>{c.label}</option>
                        {/each}
                    </select>
                    <select 
                        value={f.operator} 
                        onchange={(e) => updateFilter(i, { operator: e.currentTarget.value } as any)}
                        style="width: 75px;"
                        class="form-select"
                    >
                        <option value="is">is</option>
                        <option value="is not">is not</option>
                    </select>

                    <div style="flex: 1; display: flex;">
                        {#if getValuesForColumn((f.column as string) || '').length > 0}
                            <select 
                                multiple 
                                value={Array.isArray(f.value) ? f.value : (f.value ? [f.value] : [])} 
                                onchange={(e) => {
                                    const opts = Array.from(e.currentTarget.selectedOptions).map(o => o.value);
                                    updateFilter(i, { value: opts } as any);
                                }}
                                style="flex: 1; height: auto; min-height: 2em; padding: 2px;"
                                class="form-select"
                            >
                                {#each getValuesForColumn((f.column as string) || '') as val}
                                    <option value={val}>{val}</option>
                                {/each}
                            </select>
                        {:else}
                            <input 
                                type="text"
                                placeholder="Value..."
                                value={Array.isArray(f.value) ? f.value.join(', ') : f.value || ''}
                                oninput={(e) => updateFilter(i, { value: [e.currentTarget.value] } as any)}
                                style="flex: 1; width: 100%;"
                                class="form-input"
                            />
                        {/if}
                    </div>

                    <button class="filter-remove-button" onclick={() => removeFilter(i)} title="Remove filter" aria-label="Remove filter">
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                    </button>
                </div>
            {/each}
            <button class="filter-add-button" onclick={addFilter}>
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
                Add filter
            </button>
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
