<script lang="ts" generics="C extends string">
    import FloatingMenu from './FloatingMenu.svelte';
    import FilterGroupEditor from './filters/FilterGroupEditor.svelte';
    import FilterSortRow from './FilterSortRow.svelte';
    import { useTagOptions } from '../lib/filterOptions.svelte';
    import type { FilterGroup } from '../lib/domain';
    import { countFilters, emptyFilterGroup, type FilterColumn, type FilterCondition, type FilterOption } from '../lib/domain/filterTree';

    type SortItem = { column: C | null; direction: 'asc' | 'desc' | null };

    let {
        filters = $bindable(emptyFilterGroup<FilterCondition<C>>()),
        columns = [],
        getValuesForColumn = (_column: string): FilterOption[] => [],
        sort = $bindable([] as SortItem[]),
        sortOptions = [] as { id: C; label: string }[],
        align = 'right',
        onOpenChange,
    }: {
        filters?: FilterGroup<FilterCondition<C>>;
        columns?: FilterColumn<C>[];
        getValuesForColumn?: (column: string) => FilterOption[];
        sort?: SortItem[];
        sortOptions?: { id: C; label: string }[];
        align?: 'left' | 'right';
        onOpenChange?: (open: boolean) => void;
    } = $props();

    const tagOptions = useTagOptions();

    const badge = $derived(countFilters(filters) + sort.length);

    function valuesForColumn(column: string): FilterOption[] {
        return column === 'tags' ? tagOptions() : getValuesForColumn(column);
    }

    function addSort() {
        const id = sortOptions[0]?.id;
        if (id !== undefined) {
            sort = [...(sort ?? []), { column: id, direction: 'asc' }];
        }
    }

    function removeSort(index: number) {
        sort = sort.filter((_, i) => i !== index);
    }

    function updateSortCol(index: number, val: string) {
        if (index >= sort.length) return;
        sort[index].column = val as C;
        sort = [...sort];
    }

    function updateSortDir(index: number, val: string) {
        if (index >= sort.length) return;
        sort[index].direction = val as 'asc' | 'desc';
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

    function handleDragOver(e: DragEvent, _index: number) {
        e.preventDefault();
        if (e.dataTransfer) {
            e.dataTransfer.dropEffect = 'move';
        }
    }

    function handleDrop(e: DragEvent, index: number) {
        e.preventDefault();
        if (dragIndex !== null && dragIndex !== index) {
            const newSort = [...sort];
            const [item] = newSort.splice(dragIndex, 1);
            newSort.splice(index, 0, item);
            sort = newSort;
        }
        dragIndex = null;
    }
</script>

<FloatingMenu {align} minWidth="500px" title="View" active={badge > 0} triggerHeight="28px" {onOpenChange}>
    {#snippet trigger()}
        <span class="material-symbols-sharp view-menu-eye" aria-hidden="true">eye_tracking</span>
    {/snippet}

    <div class="view-menu-section">Filter</div>
    <div class="view-menu-filters">
        <FilterGroupEditor group={filters} {columns} {valuesForColumn} onchange={next => filters = next} />
    </div>

    {#if sortOptions.length > 0}
        <div class="view-menu-section">Sort</div>
        <div class="view-menu-sorts">
            {#each sort as s, i (i)}
                <FilterSortRow
                    draggable
                    ondragstart={(e) => handleDragStart(e, i)}
                    ondragover={(e) => handleDragOver(e, i)}
                    ondrop={(e) => handleDrop(e, i)}
                    onremove={() => removeSort(i)}
                    removeTitle="Remove Sort"
                >
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
                </FilterSortRow>
            {/each}
            <button class="view-menu-add-sort" onclick={addSort}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
                Add Sort
            </button>
        </div>
    {/if}
</FloatingMenu>

<style>
    .view-menu-eye {
        font-size: 18px;
    }

    .view-menu-section {
        color: var(--fg-dim);
        font-size: 0.8em;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        border-bottom: 1px solid var(--border);
        padding-bottom: 4px;
    }

    .view-menu-filters {
        max-height: min(70vh, 600px);
        max-width: calc(100vw - 64px);
        overflow: auto;
        padding: 2px;
    }

    .view-menu-sorts {
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    .view-menu-add-sort {
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

    .view-menu-add-sort:hover {
        opacity: 1;
    }
</style>
