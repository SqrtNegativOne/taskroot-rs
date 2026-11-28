<script lang="ts">
    import type {
        AppCalendar,
        AppEvent,
        AppTask,
        EventPatch,
        EventStatus,
        TaskPriority,
    } from '../../lib/domain';
    import { calendarsForComponent, isCalendarReadOnly } from '../../lib/domain';
    import { resolveEventColor } from '../../lib/eventColor';
    import type { EventDateFields } from './eventDates';
    import { localTimezoneIfUtc } from './format';
    import NumberInput from '../inputs/NumberInput.svelte';
    import SelectInput from '../inputs/SelectInput.svelte';
    import TagsInput from '../inputs/TagsInput.svelte';
    import DateRangePickerField from './DateRangePickerField.svelte';
    import InspectorField from './InspectorField.svelte';
    import MetaFooter from './MetaFooter.svelte';
    import RecurrenceEditor from './RecurrenceEditor.svelte';

    interface Props {
        event: AppEvent;
        tasks: AppTask[];
        calendars?: AppCalendar[];
        availableTags?: string[];
        updateEvent: (id: string, patches: EventPatch[]) => void;
        readOnly?: boolean;
        onDelete?: () => void;
        onConvert?: () => void;
    }

    let {
        event,
        tasks,
        calendars: allCalendars = [],
        availableTags = [],
        updateEvent,
        readOnly = false,
        onDelete,
        onConvert,
    }: Props = $props();

    const EVENT_STATUSES = ['confirmed', 'tentative', 'cancelled'] as const;

    let calendars = $derived(calendarsForComponent(allCalendars, 'VEVENT'));

    let calendarOptions = $derived.by(() => {
        const writable = calendars.filter((calendar) => !isCalendarReadOnly(calendar));
        const current = calendars.find((calendar) => calendar.id === event.remoteCollectionId);
        const selectable = current && isCalendarReadOnly(current)
            ? [...writable, current]
            : writable;
        return [
            { label: 'No calendar', value: '' },
            ...selectable.map((calendar) => ({
                label: calendar.summary || calendar.id,
                value: calendar.id,
                ...(calendar.color ? { color: calendar.color } : {}),
            })),
        ];
    });

    let currentCalendar = $derived(
        calendars.find((calendar) => calendar.id === event.remoteCollectionId),
    );
    let effectiveColor = $derived(resolveEventColor(event, calendars));
    let colorLabel = $derived(
        event.color ? 'Custom' : currentCalendar?.color ? 'From calendar' : 'Default',
    );

    function isEventStatus(value: string): value is EventStatus {
        return EVENT_STATUSES.some((status) => status === value);
    }

    function isTaskPriority(value: number): value is TaskPriority {
        return Number.isInteger(value) && value >= 0 && value <= 4;
    }

    function applyDates(fields: EventDateFields): void {
        const tz = !fields.isAllDay
            ? localTimezoneIfUtc([event.startTime, event.endTime], event.timezone)
            : undefined;
        updateEvent(event.id, [
            { field: 'startTime', value: fields.startTime },
            { field: 'endTime', value: fields.endTime },
            { field: 'isAllDay', value: fields.isAllDay ?? null },
            ...(tz ? [{ field: 'timezone' as const, value: tz }] : []),
        ]);
    }

    function handleStatusChange(value: string): void {
        if (!isEventStatus(value)) return;
        updateEvent(event.id, [{ field: 'status', value }]);
    }

    function handlePriorityChange(value: number): void {
        const clamped = Math.max(0, Math.min(4, value));
        if (isTaskPriority(clamped)) updateEvent(event.id, [{ field: 'priority', value: clamped }]);
    }

    function handleTagsChange(newTags: string[]): void {
        updateEvent(event.id, [{
            field: 'tags',
            value: newTags.map((name) => ({ id: crypto.randomUUID(), name })),
        }]);
    }
</script>

<InspectorField icon="calendar_month" label="Calendar" disabled={readOnly}>
    <SelectInput
        value={event.remoteCollectionId ?? ''}
        onchange={(value) => updateEvent(event.id, [{ field: 'remoteCollectionId', value: value || null }])}
        options={calendarOptions}
        variant="bare"
        disabled={readOnly}
        ariaLabel="Calendar"
    />
</InspectorField>

<InspectorField icon="palette" label="Colour" disabled={readOnly}>
    <div class="inspector-inline inspector-inline-between">
        <div class="inspector-color-field">
            <input
                type="color"
                class="inspector-color-input"
                value={effectiveColor?.slice(0, 7) ?? '#888888'}
                disabled={readOnly}
                aria-label="Event colour"
                onchange={(e) => updateEvent(event.id, [{ field: 'color', value: e.currentTarget.value }])}
            />
            <span class="inspector-readonly-value">{colorLabel}</span>
        </div>
        {#if event.color && !readOnly}
            <button
                type="button"
                class="inspector-clear"
                aria-label="Clear event colour"
                onclick={() => updateEvent(event.id, [{ field: 'color', value: null }])}
            >
                <span class="material-symbols-outlined" style="font-size: 14px;">close</span>
            </button>
        {/if}
    </div>
</InspectorField>

<InspectorField icon="check_circle" label="Status" disabled={readOnly}>
    <SelectInput
        value={event.status ?? 'confirmed'}
        onchange={handleStatusChange}
        options={[
            { label: 'Confirmed', value: 'confirmed' },
            { label: 'Tentative', value: 'tentative' },
            { label: 'Cancelled', value: 'cancelled' },
        ]}
        variant="bare"
        disabled={readOnly}
        ariaLabel="Status"
    />
</InspectorField>

<InspectorField icon="flag" label="Priority" disabled={readOnly}>
    <NumberInput
        min={0}
        max={4}
        value={event.priority ?? 0}
        onchange={handlePriorityChange}
        variant="bare"
        disabled={readOnly}
        ariaLabel="Priority"
    />
</InspectorField>

<InspectorField icon="sell" label="Tags" disabled={readOnly}>
    <TagsInput
        id={`event-tags-${event.id}`}
        tags={event.tags ? event.tags.map(t => t.name) : []}
        suggestions={availableTags}
        onchange={handleTagsChange}
        disabled={readOnly}
        variant="bare"
        ariaLabel="Tags"
    />
</InspectorField>

<InspectorField icon="schedule" label="When" disabled={readOnly}>
    <DateRangePickerField
        {event}
        disabled={readOnly}
        onchange={applyDates}
    />
</InspectorField>

<InspectorField icon="repeat" label="Repeat" disabled={readOnly}>
    <RecurrenceEditor
        value={event.rrule}
        onchange={(value) => updateEvent(event.id, [{ field: 'rrule', value: value ?? null }])}
        disabled={readOnly}
    />
</InspectorField>

<InspectorField icon="task" label="Attached task" disabled={readOnly}>
    <SelectInput
        value={event.taskId ?? ''}
        onchange={(value) => updateEvent(event.id, [{ field: 'taskId', value: value || null }])}
        options={[
            { label: 'No task attached', value: '' },
            ...tasks.map(t => ({ label: t.title, value: t.id })),
        ]}
        variant="bare"
        disabled={readOnly}
        ariaLabel="Attached task"
    />
</InspectorField>

<MetaFooter
    created={event.added}
    modified={event.updatedAt}
    onDelete={onDelete}
    deleteLabel="Delete event"
    deleteDisabled={readOnly}
    {onConvert}
    convertDisabled={readOnly}
/>
