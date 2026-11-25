<script lang="ts">
    import type { AppCalendar, AppEventFilter, AppEventFilterColumn, FilterColumn, FilterGroup } from '../lib/domain';
    import { calendarsForComponent, emptyFilterGroup } from '../lib/domain';
    import type { EventVisibility } from '../lib/eventVisibility';
    import FilterButton from './FilterButton.svelte';

    let {
        filters = $bindable(emptyFilterGroup<AppEventFilter>()),
        calendars,
        visibility = $bindable(),
        align = 'right',
        onOpenChange,
    }: {
        filters?: FilterGroup<AppEventFilter>;
        calendars: AppCalendar[];
        visibility?: EventVisibility;
        align?: 'left' | 'right';
        onOpenChange?: (open: boolean) => void;
    } = $props();

    const calendarOptions = $derived(calendarsForComponent(calendars, 'VEVENT')
        .map(calendar => ({ label: calendar.summary || calendar.id, value: calendar.id })));
    const visibilityCount = $derived(Number(visibility?.hideFrequentRecurring ?? false) + Number(visibility?.hideTimed ?? false) + Number(visibility?.hideDoneTasks ?? false));

    const filterColumns: FilterColumn<AppEventFilterColumn>[] = [
        { id: 'remotecollectionid', label: 'Calendar' },
        { id: 'title', label: 'Title' },
        { id: 'status', label: 'Status' },
        { id: 'priority', label: 'Priority' },
        { id: 'tags', label: 'Tags' },
    ];
    const statusOptions = ['confirmed', 'tentative', 'cancelled']
        .map(value => ({ label: value, value }));

    function valuesForColumn(column: string): { label: string; value: string }[] {
        if (column === 'remotecollectionid') return calendarOptions;
        if (column === 'status') return statusOptions;
        return [];
    }
</script>

<FilterButton
    bind:filters
    columns={filterColumns}
    getValuesForColumn={valuesForColumn}
    {align}
    extraFilterCount={visibilityCount}
    {onOpenChange}
>
    {#snippet extraFilters()}
        {#if visibility}
            <div class="event-visibility-filters">
                <label>
                    <input type="checkbox" bind:checked={visibility.hideFrequentRecurring} />
                    Hide daily or weekly recurring events
                </label>
                <label>
                    <input type="checkbox" bind:checked={visibility.hideTimed} />
                    Hide timed events
                </label>
                <label>
                    <input type="checkbox" bind:checked={visibility.hideDoneTasks} />
                    Hide completed tasks
                </label>
            </div>
        {/if}
    {/snippet}
</FilterButton>

<style>
    .event-visibility-filters {
        display: flex;
        flex-direction: column;
        gap: 8px;
        padding: 4px 0 8px;
        border-bottom: 1px solid var(--border);
    }

    label {
        display: flex;
        align-items: center;
        gap: 8px;
        cursor: pointer;
    }

    input {
        margin: 0;
        accent-color: var(--accent);
    }
</style>
