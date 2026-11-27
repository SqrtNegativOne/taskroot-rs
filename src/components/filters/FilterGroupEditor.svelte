<script lang="ts" generics="C extends string">
    import FilterGroupEditor from './FilterGroupEditor.svelte';
    import FilterConditionRow from './FilterConditionRow.svelte';
    import type { FilterGroup, FilterNode } from '../../lib/domain';
    import { newFilterCondition, type FilterColumn, type FilterCondition, type FilterOption } from '../../lib/domain/filterTree';

    let { group, columns, valuesForColumn, onchange, onremove }: {
        group: FilterGroup<FilterCondition<C>>;
        columns: FilterColumn<C>[];
        valuesForColumn: (column: string) => FilterOption[];
        onchange: (group: FilterGroup<FilterCondition<C>>) => void;
        onremove?: () => void;
    } = $props();

    function replaceChild(index: number, child: FilterNode<FilterCondition<C>>) {
        onchange({ ...group, children: group.children.map((current, i) => i === index ? child : current) });
    }

    function removeChild(index: number) {
        onchange({ ...group, children: group.children.filter((_, i) => i !== index) });
    }

    function addCondition() {
        const column = columns[0];
        if (!column) return;
        onchange({ ...group, children: [...group.children, { kind: 'condition', filter: newFilterCondition(column) }] });
    }

    function addGroup() {
        const column = columns[0];
        const children: FilterNode<FilterCondition<C>>[] = column ? [{ kind: 'condition', filter: newFilterCondition(column) }] : [];
        onchange({ ...group, children: [...group.children, { kind: 'group', operator: 'and', children }] });
    }
</script>

<div class="filter-group" class:nested={onremove !== undefined} role="group" aria-label={onremove ? 'Filter group' : 'Filters'}>
    {#each group.children as child, i (i)}
        <div class="filter-entry">
            <span class="connector">
                {#if i === 1}
                    <select
                        aria-label="Group operator"
                        value={group.operator}
                        onchange={event => onchange({ ...group, operator: event.currentTarget.value === 'or' ? 'or' : 'and' })}
                        class="form-select"
                    >
                        <option value="and">AND</option>
                        <option value="or">OR</option>
                    </select>
                {:else}
                    {i === 0 ? 'Where' : group.operator.toUpperCase()}
                {/if}
            </span>
            <div class="entry-content">
                {#if child.kind === 'condition'}
                    <FilterConditionRow
                        filter={child.filter}
                        {columns}
                        {valuesForColumn}
                        onchange={filter => replaceChild(i, { kind: 'condition', filter })}
                        onremove={() => removeChild(i)}
                    />
                {:else}
                    <FilterGroupEditor
                        group={child}
                        {columns}
                        {valuesForColumn}
                        onchange={next => replaceChild(i, { kind: 'group', ...next })}
                        onremove={() => removeChild(i)}
                    />
                {/if}
            </div>
        </div>
    {/each}
    {#if group.children.length === 0}
        <span class="empty-hint">No conditions — this group is ignored.</span>
    {/if}
    <div class="group-actions">
        <button onclick={addCondition} disabled={columns.length === 0}>+ Add filter</button>
        <button onclick={addGroup} disabled={columns.length === 0}>+ Add group</button>
        {#if onremove}
            <button class="clear" onclick={onremove}>Remove filter group</button>
        {:else if group.children.length > 0}
            <button class="clear" onclick={() => onchange({ ...group, children: [] })}>Clear filters</button>
        {/if}
    </div>
</div>

<style>
    .filter-group { display: flex; flex-direction: column; gap: 8px; min-width: 440px; }
    .nested { border: 1px solid var(--border); border-radius: 0; padding: 10px; background: var(--bg); }
    .connector select { width: 100%; }
    .filter-entry { display: flex; align-items: flex-start; gap: 6px; }
    .connector { flex: 0 0 58px; min-height: 28px; display: flex; align-items: center; font-size: 0.85em; color: var(--fg-muted); }
    .entry-content { flex: 1; min-width: 0; }
    .group-actions { display: flex; gap: 12px; padding-left: 64px; }
    button { background: transparent; border: none; color: var(--fg); cursor: pointer; padding: 4px; font-size: 0.9em; opacity: 0.7; }
    button:hover { opacity: 1; }
    button:disabled { cursor: default; opacity: 0.4; }
    .clear { margin-left: auto; }
    .empty-hint { font-size: 0.85em; opacity: 0.6; padding-left: 64px; }
</style>
