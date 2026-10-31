<script lang="ts" generics="T">
    import FloatingMenu from './FloatingMenu.svelte';
    import FilterSortRow from './FilterSortRow.svelte';

    type SortItem<T> = { column: T | null; direction: 'asc' | 'desc' | null };

    let {
        sort = $bindable([]),
        sortOptions = [],
        align = "left",
    }: {
        sort?: SortItem<T>[];
        sortOptions?: {id: T, label: string}[];
        align?: "left" | "right";
    } = $props();

    function addSort() {
        const id = sortOptions[0]?.id;
        if (id !== undefined) {
            sort = [...(sort ?? []), { column: id, direction: 'asc' }];
        }
    }

    function removeSort(index: number) {
        if (!sort) return;
        sort = sort.filter((_, i) => i !== index);
    }

    function updateSortCol(index: number, val: string) {
        if (!sort || index >= sort.length) return;
        sort[index].column = val as T;
        sort = [...sort];
    }

    function updateSortDir(index: number, val: string) {
        if (!sort || index >= sort.length) return;
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
        if (sort && dragIndex !== null && dragIndex !== index) {
            const newSort = [...sort];
            const [item] = newSort.splice(dragIndex, 1);
            newSort.splice(index, 0, item);
            sort = newSort;
        }
        dragIndex = null;
    }
</script>

{#if sortOptions.length > 0}
    <FloatingMenu
        align={align}
        minWidth="280px"
        title="Sort"
        active={sort.length > 0}
        badge={sort.length}
        addLabel="Add Sort"
        onadd={addSort}
    >
        {#snippet trigger()}
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m3 16 4 4 4-4"/><path d="M7 20V4"/><path d="m21 8-4-4-4 4"/><path d="M17 4v16"/></svg>
        {/snippet}

        {#if sort}
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
        {/if}
    </FloatingMenu>
{/if}
