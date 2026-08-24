<script lang="ts">
    import type { AppEvent, AppTask } from '../../lib/domain';
    import { getFormattedDate, getFormattedTime } from './format';
    import SelectInput from '../inputs/SelectInput.svelte';
    import TimeInput from '../inputs/TimeInput.svelte';

    interface Props {
        event: AppEvent;
        tasks: AppTask[];
        updateEvent: (id: string, e: (e: AppEvent) => AppEvent) => void;
    }

    let { event, tasks, updateEvent }: Props = $props();

    function updateEventDate(field: 'startTime' | 'endTime', dateStr: string): void {
        if (!event[field]) return;
        const timeStr = event[field].split('T')[1] ?? '00:00:00';
        updateEvent(event.id, (e) => ({ ...e, [field]: `${dateStr}T${timeStr}` }));
    }

    function updateEventTime(field: 'startTime' | 'endTime', timeStr: string): void {
        if (!event[field]) return;
        const dateStr = event[field].split('T')[0];
        updateEvent(event.id, (e) => ({ ...e, [field]: `${dateStr}T${timeStr}:00` }));
    }

    function handleAttachmentChange(value: string): void {
        updateEvent(event.id, (e) => ({ ...e, taskId: value || undefined }));
    }

    function handleRruleChange(value: string): void {
        updateEvent(event.id, (e) => ({ ...e, rrule: value || undefined }));
    }
</script>

<div class="inspector-field">
    <label for={`attach-${event.id}`}>Task Attachment</label>
    <SelectInput
        value={event.taskId ?? ''}
        onchange={handleAttachmentChange}
        options={[
            { label: '-- No task attached --', value: '' },
            ...tasks.map(t => ({ label: t.title, value: t.id }))
        ]}
    />
</div>

<div class="inspector-field inspector-field-group">
    <label for={`rrule-${event.id}`}>Repeat (RRULE)</label>
    <input
        id={`rrule-${event.id}`}
        type="text"
        placeholder="Custom RRULE (e.g. FREQ=WEEKLY)"
        value={event.rrule ?? ''}
        onchange={(e) => handleRruleChange(e.currentTarget.value)}
    />
</div>

<div class="inspector-field-group">
    <label for={`start-d-${event.id}`}>Start</label>
    <div style="display: flex; gap: 8px;">
        <input
            id={`start-d-${event.id}`}
            type="date"
            class="inspector-date-input"
            value={getFormattedDate(event.startTime)}
            onchange={(e) => updateEventDate('startTime', e.currentTarget.value)}
        />
        {#if event.startTime.includes('T')}
            <TimeInput
                class="inspector-date-input"
                value={getFormattedTime(event.startTime)}
                onchange={(val) => updateEventTime('startTime', val)}
            />
        {/if}
    </div>
</div>

<div class="inspector-field-group" style="margin-top: 8px;">
    <label for={`end-d-${event.id}`}>End</label>
    <div style="display: flex; gap: 8px;">
        <input
            id={`end-d-${event.id}`}
            type="date"
            class="inspector-date-input"
            value={getFormattedDate(event.endTime)}
            onchange={(e) => updateEventDate('endTime', e.currentTarget.value)}
        />
        {#if event.endTime.includes('T')}
            <TimeInput
                class="inspector-date-input"
                value={getFormattedTime(event.endTime)}
                onchange={(val) => updateEventTime('endTime', val)}
            />
        {/if}
    </div>
</div>
