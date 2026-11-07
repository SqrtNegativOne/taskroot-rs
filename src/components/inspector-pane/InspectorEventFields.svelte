<script lang="ts">
    import type { AppCalendar, AppEvent, AppTask, EventStatus } from '../../lib/domain';
    import { calendarsForComponent, isCalendarReadOnly } from '../../lib/domain';
    import { resolveEventColor } from '../../lib/eventColor';
    import { activeCalendars } from '../../lib/queries';
    import { useAutoQuery } from '../../lib/safeInvoke.svelte';
    import { getFormattedTime } from './format';
    import {
        eventEndDate,
        eventEndDateInclusive,
        eventIsAllDay,
        eventStartDate,
        toAllDay,
        toTimed,
        withEndDate,
        withEndTime,
        withStartDate,
        withStartTime,
        type EventDateFields,
    } from './eventDates';
    import DateField from './DateField.svelte';
    import InspectorField from './InspectorField.svelte';
    import MetaFooter from './MetaFooter.svelte';
    import RecurrenceEditor from './RecurrenceEditor.svelte';
    import SelectInput from '../inputs/SelectInput.svelte';
    import TimeInput from '../inputs/TimeInput.svelte';
    import ToggleSwitch from '../inputs/ToggleSwitch.svelte';

    interface Props {
        event: AppEvent;
        tasks: AppTask[];
        updateEvent: (id: string, update: (event: AppEvent) => AppEvent) => void;
        readOnly?: boolean;
    }

    let { event, tasks, updateEvent, readOnly = false }: Props = $props();

    const EVENT_STATUSES = ['confirmed', 'tentative', 'cancelled'] as const;

    const calendarsQuery = useAutoQuery<AppCalendar[]>('get_active_calendars', activeCalendars);
    let calendars = $derived(calendarsForComponent(calendarsQuery.data ?? [], 'VEVENT'));

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

    let allDay = $derived(eventIsAllDay(event));
    let startDate = $derived(eventStartDate(event) ?? '');
    let timedEndDate = $derived(eventEndDate(event) ?? startDate);
    let endDate = $derived(allDay ? (eventEndDateInclusive(event) ?? startDate) : timedEndDate);

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

    function applyDates(change: (current: AppEvent) => EventDateFields): void {
        updateEvent(event.id, (current) => ({ ...current, ...change(current) }));
    }

    function handleAllDayChange(checked: boolean): void {
        applyDates((current) => (checked ? toAllDay(current) : toTimed(current)));
    }

    function handleStatusChange(value: string): void {
        if (!isEventStatus(value)) return;
        updateEvent(event.id, (current) => ({ ...current, status: value }));
    }

    function handleCalendarChange(value: string): void {
        updateEvent(event.id, (current) => ({
            ...current,
            remoteCollectionId: value || undefined,
        }));
    }

    function handleColorChange(value: string): void {
        updateEvent(event.id, (current) => ({ ...current, color: value }));
    }

    function handleColorClear(): void {
        updateEvent(event.id, (current) => ({ ...current, color: undefined }));
    }

    function handleAttachmentChange(value: string): void {
        updateEvent(event.id, (current) => ({ ...current, taskId: value || undefined }));
    }

    function handleRecurrenceChange(value: string | undefined): void {
        updateEvent(event.id, (current) => ({ ...current, rrule: value }));
    }
</script>

<InspectorField icon="calendar_month" label="Calendar" disabled={readOnly}>
    <SelectInput
        value={event.remoteCollectionId ?? ''}
        onchange={handleCalendarChange}
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
                onchange={(e) => handleColorChange(e.currentTarget.value)}
            />
            <span class="inspector-readonly-value">{colorLabel}</span>
        </div>
        {#if event.color && !readOnly}
            <button
                type="button"
                class="inspector-clear"
                aria-label="Clear event colour"
                onclick={handleColorClear}
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

<InspectorField icon="schedule" label="All day" disabled={readOnly}>
    <div class="inspector-inline inspector-inline-between">
        <span class="inspector-readonly-value">{allDay ? 'All day' : 'Timed'}</span>
        <ToggleSwitch
            checked={allDay}
            onchange={handleAllDayChange}
            ariaLabel="All day event"
            disabled={readOnly}
        />
    </div>
</InspectorField>

<InspectorField icon="play_circle" label="Starts" disabled={readOnly}>
    <div class="inspector-inline">
        <DateField
            value={startDate}
            placeholder="No date"
            ariaLabel="Start date"
            disabled={readOnly}
            onchange={(value) => applyDates((current) => withStartDate(current, value))}
        />
        {#if !allDay}
            <TimeInput
                value={getFormattedTime(event.startTime)}
                onchange={(value) => applyDates((current) => withStartTime(current, value))}
                variant="bare"
                disabled={readOnly}
                ariaLabel="Start time"
            />
        {/if}
    </div>
</InspectorField>

<InspectorField icon="stop_circle" label="Ends" disabled={readOnly}>
    <div class="inspector-inline">
        <DateField
            value={endDate}
            placeholder="Same day"
            ariaLabel={allDay ? 'End date (inclusive)' : 'End date'}
            disabled={readOnly}
            min={startDate}
            onchange={(value) => applyDates((current) => withEndDate(current, value))}
        />
        {#if !allDay}
            <TimeInput
                value={getFormattedTime(event.endTime)}
                onchange={(value) => applyDates((current) => withEndTime(current, value))}
                variant="bare"
                disabled={readOnly}
                ariaLabel="End time"
            />
        {/if}
    </div>
</InspectorField>

<InspectorField icon="repeat" label="Repeat" disabled={readOnly}>
    <RecurrenceEditor
        value={event.rrule}
        onchange={handleRecurrenceChange}
        disabled={readOnly}
    />
</InspectorField>

<InspectorField icon="link" label="Attached task" disabled={readOnly}>
    <SelectInput
        value={event.taskId ?? ''}
        onchange={handleAttachmentChange}
        options={[
            { label: 'No task attached', value: '' },
            ...tasks.map(t => ({ label: t.title, value: t.id })),
        ]}
        variant="bare"
        disabled={readOnly}
        ariaLabel="Attached task"
    />
</InspectorField>

<MetaFooter created={event.added} modified={event.updatedAt} />
