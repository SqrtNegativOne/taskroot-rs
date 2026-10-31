<script lang="ts" generics="F extends { column?: string | null, operator?: string | null, value?: unknown }">
    import FloatingMenu from './FloatingMenu.svelte';
    import FilterSortRow from './FilterSortRow.svelte';
    import MultiSelect from './inputs/MultiSelect.svelte';

    let {
        filters = $bindable([]),
        columns = [],
        getValuesForColumn = (_col: string): {label: string, value: string}[] => [],
        align = "left",
    }: {
        filters?: F[];
        columns?: {id: string, label: string}[];
        getValuesForColumn?: (col: string) => {label: string, value: string}[];
        align?: "left" | "right";
    } = $props();

    function addFilter() {
        const id = columns[0]?.id ?? 'status';
        const blankFilter = {} as F;
        filters = [...filters, { ...blankFilter, column: id, operator: 'is', value: [] }];
    }

    function removeFilter(index: number) {
        filters = filters.filter((_, i) => i !== index);
    }

    function updateFilter(index: number, updates: { column?: string | null, operator?: string | null, value?: unknown }) {
        if (index >= filters.length) return;
        Object.assign(filters[index], updates);
        filters = [...filters];
    }
</script>

<FloatingMenu
    align={align}
    minWidth="460px"
    title="Filter"
    active={filters.length > 0}
    badge={filters.length}
    addLabel="Add filter"
    onadd={addFilter}
>
    {#snippet trigger()}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"/></svg>
    {/snippet}

    {#each filters as f, i (i)}
        {@const column = f.column ?? ''}
        {@const availableValues = getValuesForColumn(column)}
        <FilterSortRow onremove={() => removeFilter(i)} removeTitle="Remove filter">
            <select
                value={f.column}
                onchange={(e) => {
                    const val = e.currentTarget.value;
                    if (val === 'status' || val === 'priority' || val === 'tag') {
                        updateFilter(i, { column: val, value: [] });
                    }
                }}
                style="flex: 0 0 auto; width: 110px;"
                class="form-select"
            >
                {#each columns as c (c.id)}
                    <option value={c.id}>{c.label}</option>
                {/each}
            </select>
            <select
                value={f.operator}
                onchange={(e) => updateFilter(i, { operator: e.currentTarget.value })}
                style="width: 75px;"
                class="form-select"
            >
                <option value="is">is</option>
                <option value="is not">is not</option>
            </select>

            <div style="flex: 1; min-width: 0; display: flex;">
                {#if availableValues.length > 0}
                    <MultiSelect
                        options={availableValues}
                        values={Array.isArray(f.value) ? f.value as string[] : (f.value != null ? [f.value as string] : [])}
                        onchange={(val) => updateFilter(i, { value: val })}
                        style="flex: 1; min-width: 0;"
                    />
                {:else}
                    <input
                        type="text"
                        placeholder="Value..."
                        value={Array.isArray(f.value) ? f.value.join(', ') : (f.value ?? '')}
                        oninput={(e) => updateFilter(i, { value: [e.currentTarget.value] })}
                        style="flex: 1; width: 100%;"
                        class="form-input"
                    />
                {/if}
            </div>
        </FilterSortRow>
    {/each}
</FloatingMenu>
