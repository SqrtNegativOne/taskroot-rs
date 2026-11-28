<script lang="ts">
    import type {
        AppCalendar,
        AppTask,
        AppTaskStatus,
        TaskAlarmTrigger,
        TaskPatch,
        TaskPriority,
    } from '../../lib/domain';
    import { calendarsForComponent, DEFAULT_TASK_PRIORITY } from '../../lib/domain';
    import { formatTimestamp, formatTrackedDuration, localTimezoneIfUtc } from './format';
    import NumberInput from '../inputs/NumberInput.svelte';
    import SelectInput from '../inputs/SelectInput.svelte';
    import TagsInput from '../inputs/TagsInput.svelte';
    import DatePickerField from './DatePickerField.svelte';
    import InspectorField from './InspectorField.svelte';
    import MetaFooter from './MetaFooter.svelte';
    import RecurrenceEditor from './RecurrenceEditor.svelte';

    interface Props {
        task: AppTask;
        tasks: AppTask[];
        calendars?: AppCalendar[];
        availableTags?: string[];
        updateTask: (id: string, patches: TaskPatch[]) => void;
        readOnly?: boolean;
        onDelete?: () => void;
    }

    let {
        task,
        tasks,
        calendars = [],
        availableTags = [],
        updateTask,
        readOnly = false,
        onDelete,
    }: Props = $props();

    const STATUSES = ['todo', 'next-up', 'done', 'cancelled'] as const;

    let listOptions = $derived.by(() => {
        const todos = calendarsForComponent(calendars, 'VTODO');
        const current = todos.find(c => c.id === task.remoteCollectionId);
        const writable = todos.filter(c => c.writable);
        const selectable = current && !current.writable ? [...writable, current] : writable;
        const mapped = selectable.map(c => ({
            label: c.summary || c.id,
            value: c.id,
            ...(c.color ? { color: c.color } : {}),
        }));
        if (!task.remoteCollectionId) {
            return [{ label: 'Default list', value: '' }, ...mapped];
        }
        return mapped;
    });

    let parentOptions = $derived([
        { label: 'None', value: '' },
        ...tasks.filter(t => t.id !== task.id).map(t => ({ label: t.title, value: t.id })),
    ]);

    let dependencyNames = $derived(
        (task.dependencies ?? []).map(id => tasks.find(c => c.id === id)?.title ?? id),
    );

    function isAppTaskStatus(value: string): value is AppTaskStatus {
        return STATUSES.some(status => status === value);
    }

    function isTaskPriority(value: number): value is TaskPriority {
        return Number.isInteger(value) && value >= 0 && value <= 4;
    }

    function handleStatusChange(value: string): void {
        if (!isAppTaskStatus(value)) return;
        updateTask(task.id, [{ field: 'status', value }]);
    }

    function handlePriorityChange(value: number): void {
        const clamped = Math.max(0, Math.min(4, value));
        if (isTaskPriority(clamped)) updateTask(task.id, [{ field: 'priority', value: clamped }]);
    }

    function handleDateTimeChange(field: 'start' | 'due', value: string | null): void {
        const tz = value?.includes('T')
            ? localTimezoneIfUtc([field === 'start' ? task.start : task.due], task.timezone)
            : undefined;
        updateTask(task.id, [
            field === 'start' ? { field: 'start', value } : { field: 'due', value },
            ...(tz ? [{ field: 'timezone' as const, value: tz }] : []),
        ]);
    }

    function handleTagsChange(newTags: string[]): void {
        updateTask(task.id, [{
            field: 'tags',
            value: newTags.map(name => ({ id: crypto.randomUUID(), name })),
        }]);
    }

    function formatMinutes(minutes: number): string {
        if (minutes === 0) return 'At';
        if (minutes % 1440 === 0) return `${minutes / 1440}d`;
        if (minutes % 60 === 0) return `${minutes / 60}h`;
        return `${minutes}m`;
    }

    function describeReminder(alarm: TaskAlarmTrigger): string {
        if (alarm.kind === 'absolute') return alarm.at;
        return `${formatMinutes(alarm.minutesBefore)} before ${alarm.anchor}`;
    }
</script>

<InspectorField icon="check_circle" label="Status" disabled={readOnly}>
    <SelectInput
        value={task.status ?? 'todo'}
        onchange={handleStatusChange}
        options={[
            { label: 'Todo', value: 'todo' },
            { label: 'Next Up', value: 'next-up' },
            { label: 'Done', value: 'done' },
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
        value={task.priority ?? DEFAULT_TASK_PRIORITY}
        onchange={handlePriorityChange}
        variant="bare"
        disabled={readOnly}
        ariaLabel="Priority"
    />
</InspectorField>

<InspectorField icon="list" label="List" disabled={readOnly}>
    <SelectInput
        value={task.remoteCollectionId ?? ''}
        onchange={(value) => updateTask(task.id, [{ field: 'remoteCollectionId', value: value || null }])}
        options={listOptions}
        variant="bare"
        disabled={readOnly}
        ariaLabel="List"
    />
</InspectorField>

<InspectorField icon="calendar_month" label="Start" disabled={readOnly}>
    <DatePickerField
        value={task.start}
        timezone={task.timezone}
        placeholder="No start date"
        ariaLabel="Start date"
        timeAriaLabel="Start time"
        disabled={readOnly}
        onchange={(value) => handleDateTimeChange('start', value)}
    />
</InspectorField>

<InspectorField icon="hourglass" label="Deadline" disabled={readOnly}>
    <DatePickerField
        value={task.due}
        timezone={task.timezone}
        placeholder="No due date"
        ariaLabel="Due date"
        timeAriaLabel="Due time"
        disabled={readOnly}
        onchange={(value) => handleDateTimeChange('due', value)}
    />
</InspectorField>

<InspectorField icon="timer" label="Estimate" disabled={readOnly}>
    <div class="inspector-inline">
        <NumberInput
            min={0}
            value={task.est ?? ''}
            onchange={(value) => updateTask(task.id, [{ field: 'est', value: value || null }])}
            variant="bare"
            disabled={readOnly}
            ariaLabel="Estimate in minutes"
            placeholder="No estimate"
        />
        <span class="inspector-suffix">min</span>
    </div>
</InspectorField>

<InspectorField icon="sell" label="Tags" disabled={readOnly}>
    <TagsInput
        id={`tags-${task.id}`}
        tags={task.tags ? task.tags.map(t => t.name) : []}
        suggestions={availableTags}
        onchange={handleTagsChange}
        disabled={readOnly}
        variant="bare"
        ariaLabel="Tags"
    />
</InspectorField>

<InspectorField icon="account_tree" label="Parent task" disabled={readOnly}>
    <SelectInput
        value={task.parentTask ?? ''}
        onchange={(value) => updateTask(task.id, [{ field: 'parentTask', value: value || null }])}
        options={parentOptions}
        variant="bare"
        disabled={readOnly}
        ariaLabel="Parent task"
    />
</InspectorField>

<InspectorField icon="repeat" label="Repeat" disabled={readOnly}>
    <RecurrenceEditor
        value={task.rrule}
        onchange={(value) => updateTask(task.id, [{ field: 'rrule', value: value ?? null }])}
        disabled={readOnly}
    />
</InspectorField>

{#if dependencyNames.length > 0}
    <InspectorField icon="hub" label="Depends on">
        <span class="inspector-readonly-value">{dependencyNames.join(', ')}</span>
    </InspectorField>
{/if}

{#if task.alarms && task.alarms.length > 0}
    <InspectorField icon="notifications" label="Reminders">
        <span class="inspector-readonly-value">
            {task.alarms.map(describeReminder).join(', ')}
        </span>
    </InspectorField>
{/if}

{#if task.timeSpent && task.timeSpent > 0}
    <InspectorField icon="schedule" label="Time spent">
        <span class="inspector-readonly-value">{formatTrackedDuration(task.timeSpent)}</span>
    </InspectorField>
{/if}

{#if task.completedAt}
    <InspectorField icon="task_alt" label="Completed">
        <span class="inspector-readonly-value">{formatTimestamp(task.completedAt)}</span>
    </InspectorField>
{/if}

<MetaFooter
    created={task.added}
    modified={task.updatedAt}
    onDelete={onDelete}
    deleteLabel="Delete task"
    deleteDisabled={readOnly}
/>
