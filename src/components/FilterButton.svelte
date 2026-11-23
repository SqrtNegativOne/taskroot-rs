<script lang="ts" generics="C extends string">
    import type { Snippet } from 'svelte';
    import FloatingMenu from './FloatingMenu.svelte';
    import FilterGroupEditor from './filters/FilterGroupEditor.svelte';
    import { useAutoQuery } from '../lib/safeInvoke.svelte';
    import type { FilterGroup } from '../lib/bindings/FilterGroup.generated';
    import { countFilters, emptyFilterGroup, type FilterColumn, type FilterCondition, type FilterOption } from '../lib/domain/filterTree';

    let {
        filters = $bindable(emptyFilterGroup<FilterCondition<C>>()),
        columns = [],
        getValuesForColumn = (_column: string): FilterOption[] => [],
        align = 'left',
        extraFilters,
        extraFilterCount = 0,
        onOpenChange,
    }: {
        filters?: FilterGroup<FilterCondition<C>>;
        columns?: FilterColumn<C>[];
        getValuesForColumn?: (column: string) => FilterOption[];
        align?: 'left' | 'right';
        extraFilters?: Snippet;
        extraFilterCount?: number;
        onOpenChange?: (open: boolean) => void;
    } = $props();

    const tagsQuery = useAutoQuery<string[]>('list_tags', () => ({}));
    const tagOptions = $derived((tagsQuery.data ?? []).map(name => ({ label: name, value: name })));
    const badge = $derived(countFilters(filters) + extraFilterCount);

    function valuesForColumn(column: string): FilterOption[] {
        return column === 'tags' ? tagOptions : getValuesForColumn(column);
    }
</script>

<FloatingMenu {align} minWidth="500px" title="Filter" active={filters.children.length > 0 || extraFilterCount > 0} {badge} {onOpenChange}>
    {#snippet trigger()}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"/></svg>
    {/snippet}
    {@render extraFilters?.()}
    <div class="filter-tree-scroll">
        <FilterGroupEditor group={filters} {columns} {valuesForColumn} onchange={next => filters = next} />
    </div>
</FloatingMenu>

<style>
    .filter-tree-scroll { max-height: min(70vh, 600px); max-width: calc(100vw - 64px); overflow: auto; padding: 2px; }
</style>
