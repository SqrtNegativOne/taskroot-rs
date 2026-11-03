<script lang="ts">
    import type { AppCalendar, AppEventFilter } from '../lib/domain';
    import { calendarsForComponent } from '../lib/domain';
    import type { EventVisibility } from '../lib/eventVisibility';
    import FilterButton from './FilterButton.svelte';

    let {
        filters = $bindable([]),
        calendars,
        visibility = $bindable(),
        align = 'right',
    }: {
        filters?: AppEventFilter[];
        calendars: AppCalendar[];
        visibility?: EventVisibility;
        align?: 'left' | 'right';
    } = $props();

    const calendarOptions = $derived(calendarsForComponent(calendars, 'VEVENT')
        .map(calendar => ({ label: calendar.summary || calendar.id, value: calendar.id })));
    const visibilityCount = $derived(Number(visibility?.hideRecurring ?? false) + Number(visibility?.hideTimed ?? false));
</script>

<FilterButton
    bind:filters
    columns={[{ id: 'remotecollectionid', label: 'Calendar' }]}
    getValuesForColumn={(column) => column === 'remotecollectionid' ? calendarOptions : []}
    {align}
    extraFilterCount={visibilityCount}
>
    {#snippet extraFilters()}
        {#if visibility}
            <div class="event-visibility-filters">
                <label>
                    <input type="checkbox" bind:checked={visibility.hideRecurring} />
                    Hide recurring events
                </label>
                <label>
                    <input type="checkbox" bind:checked={visibility.hideTimed} />
                    Hide timed events
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
