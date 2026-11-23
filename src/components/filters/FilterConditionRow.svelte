<script lang="ts" generics="C extends string">
    import FilterSortRow from '../FilterSortRow.svelte';
    import MultiSelect from '../inputs/MultiSelect.svelte';
    import { filterStrings, operatorsForColumn, newFilterCondition, type FilterColumn, type FilterCondition, type FilterOption } from '../../lib/domain/filterTree';

    let { filter, columns, valuesForColumn, onchange, onremove }: {
        filter: FilterCondition<C>;
        columns: FilterColumn<C>[];
        valuesForColumn: (column: string) => FilterOption[];
        onchange: (filter: FilterCondition<C>) => void;
        onremove: () => void;
    } = $props();

    const column = $derived(columns.find(candidate => candidate.id === filter.column));
    const options = $derived(valuesForColumn(filter.column ?? ''));
    const values = $derived(filterStrings(filter.value));
</script>

<FilterSortRow {onremove} removeTitle="Remove filter">
    <select
        aria-label="Filter property"
        value={filter.column ?? ''}
        onchange={event => {
            const selected = columns.find(candidate => candidate.id === event.currentTarget.value);
            if (selected) onchange(newFilterCondition(selected));
        }}
        class="form-select property"
    >
        {#each columns as candidate (candidate.id)}
            <option value={candidate.id}>{candidate.label}</option>
        {/each}
    </select>
    <select
        aria-label="Filter operator"
        value={filter.operator ?? 'is'}
        onchange={event => {
            const operator = operatorsForColumn(column).find(candidate => candidate === event.currentTarget.value);
            if (operator) onchange({ ...filter, operator });
        }}
        class="form-select operator"
    >
        {#each operatorsForColumn(column) as operator (operator)}
            <option value={operator}>{operator}</option>
        {/each}
    </select>
    <div class="value">
        {#if filter.column === 'tags' || options.length > 0}
            <MultiSelect {options} {values} onchange={value => onchange({ ...filter, value })} style="flex: 1; min-width: 0;" />
        {:else}
            <input
                aria-label="Filter value"
                type="text"
                placeholder={column?.filterType === 'Date' ? 'YYYY-MM-DD or today' : 'Value...'}
                value={values.join(', ')}
                oninput={event => onchange({ ...filter, value: [event.currentTarget.value] })}
                class="form-input"
            />
        {/if}
    </div>
</FilterSortRow>

<style>
    .property { flex: 0 0 110px; width: 110px; }
    .operator { flex: 0 0 110px; width: 110px; }
    .value { flex: 1; min-width: 100px; display: flex; }
    input { flex: 1; width: 100%; min-width: 0; }
</style>
