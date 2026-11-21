<script lang="ts" generics="F extends { column?: string | null, operator?: string | null, value?: unknown }">
    import type { Snippet } from 'svelte';
    import FloatingMenu from './FloatingMenu.svelte';
    import FilterSortRow from './FilterSortRow.svelte';
    import MultiSelect from './inputs/MultiSelect.svelte';
    import { useAutoQuery } from '../lib/safeInvoke.svelte';
    import type { FilterType } from '../lib/bindings/FilterType.generated';

    type FilterColumn = { id: string; label: string; filterType?: FilterType };

    const TEXT_OPERATORS = ['is', 'is not'] as const;
    const DATE_OPERATORS = ['is', 'is not', 'before', 'after', 'on or before', 'on or after'] as const;

    let {
        filters = $bindable([]),
        columns = [],
        getValuesForColumn = (_col: string): {label: string, value: string}[] => [],
        align = "left",
        extraFilters,
        extraFilterCount = 0,
    }: {
        filters?: F[];
        columns?: FilterColumn[];
        getValuesForColumn?: (col: string) => {label: string, value: string}[];
        align?: "left" | "right";
        extraFilters?: Snippet;
        extraFilterCount?: number;
    } = $props();

    function isDateColumn(column: string): boolean {
        return columns.find((candidate) => candidate.id === column)?.filterType === 'Date';
    }

    function operatorsFor(column: string): readonly string[] {
        return isDateColumn(column) ? DATE_OPERATORS : TEXT_OPERATORS;
    }

    const tagsQuery = useAutoQuery<string[]>('list_tags', () => ({}));
    const tagOptions = $derived((tagsQuery.data ?? []).map((name) => ({ label: name, value: name })));

    function availableValuesFor(column: string): { label: string; value: string }[] {
        if (column === 'tags') return tagOptions;
        return getValuesForColumn(column);
    }

    function addFilter() {
        const id = columns[0]?.id ?? 'status';
        const blankFilter = {} as F;
        filters = [...filters, {
            ...blankFilter,
            column: id,
            operator: operatorsFor(id)[0] ?? 'is',
            value: [],
        }];
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
    active={filters.length + extraFilterCount > 0}
    badge={filters.length + extraFilterCount}
    addLabel="Add filter"
    onadd={addFilter}
>
    {#snippet trigger()}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"/></svg>
    {/snippet}

    {@render extraFilters?.()}

    {#each filters as f, i (i)}
        {@const column = f.column ?? ''}
        {@const availableValues = availableValuesFor(column)}
        <FilterSortRow onremove={() => removeFilter(i)} removeTitle="Remove filter">
            <select
                value={f.column}
                onchange={(e) => {
                    const column = e.currentTarget.value;
                    updateFilter(i, {
                        column,
                        operator: operatorsFor(column)[0] ?? 'is',
                        value: [],
                    });
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
                style="width: 110px;"
                class="form-select"
            >
                {#each operatorsFor(column) as operator (operator)}
                    <option value={operator}>{operator}</option>
                {/each}
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
                        placeholder={isDateColumn(column) ? 'YYYY-MM-DD or today' : 'Value...'}
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
