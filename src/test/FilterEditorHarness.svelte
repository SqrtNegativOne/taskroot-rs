<script lang="ts">
    import { untrack } from 'svelte';
    import FilterButton from '../components/FilterButton.svelte';
    import { emptyFilterGroup, type FilterCondition, type FilterGroup, type FilterColumn } from '../lib/domain';

    let { initial = emptyFilterGroup<FilterCondition>(), columns }: {
        initial?: FilterGroup<FilterCondition>;
        columns: FilterColumn[];
    } = $props();
    let filters = $state(untrack(() => initial));
</script>

<FilterButton bind:filters {columns} />
<pre data-testid="filters">{JSON.stringify(filters)}</pre>
