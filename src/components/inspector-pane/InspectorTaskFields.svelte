<script lang="ts">
    import type {
        AppCalendar,
        AppTask,
        AppTaskStatus,
        TaskAlarmTrigger,
    } from '../../lib/domain';
    import type { TaskPriority } from '../../lib/bindings/TaskPriority.generated';
    import { calendarsForComponent } from '../../lib/domain';
    import { activeCalendars } from '../../lib/queries';
    import { useAutoQuery } from '../../lib/safeInvoke.svelte';
    import { formatTimestamp, getFormattedDate, getFormattedTime, withDate, withTime } from './format';
    import NumberInput from '../inputs/NumberInput.svelte';
    import SelectInput from '../inputs/SelectInput.svelte';
    import TagsInput from '../inputs/TagsInput.svelte';
    import TimeInput from '../inputs/TimeInput.svelte';
    import DateField from './DateField.svelte';
    import InspectorField from './InspectorField.svelte';
    import MetaFooter from './MetaFooter.svelte';
    import RecurrenceEditor from './RecurrenceEditor.svelte';

    interface Props {
        task: AppTask;
        tasks: AppTask[];
        updateTask: (id: string, update: (task: AppTask) => AppTask) => void;
        readOnly?: boolean;
        onDelete?: () => void;
    }

    let { task, tasks, updateTask, readOnly = false, onDelete }: Props = $props();

    const calendarsQuery = useAutoQuery<AppCalendar[]>('get_active_calendars', activeCalendars);

    let listOptions = $derived.by(() => {
        const todos = calendarsForComponent(calendarsQuery.data ?? [], 'VTODO');
        const current = todos.find(c => c.id === task.remoteCollectionId);
        const selectable = current && !current.writable
            ? [...todos.filter(c => c.writable), current]
            : todos.filter(c => c.writable);
        if (!task.remoteCollectionId) {
            return [
                { label: 'Default list', value: '' },
                ...selectable.map(c => ({ label: c.summary || c.id, value: c.id })),
            ];
        }
        if (current && !selectable.some(c => c.id === current.id)) {
            return [
                { label: current.summary || current.id, value: current.id },
                ...selectable.map(c => ({ label: c.summary || c.id, value: c.id })),
            ];
        }
        return selectable.map(c => ({
            label: c.summary || c.id,
            value: c.id,
            ...(c.color ? { color: c.color } : {}),
        }));
    });

    let parentOptions = $derived([
        { label: 'None', value: '' },
        ...tasks.filter(t => t.id !== task.id).map(t => ({ label: t.title, value: t.id })),
    ]);

    let dependencyNames = $derived(
        (task.dependencies ?? []).map((id) => tasks.find((candidate) => candidate.id === id)?.title ?? id),
    );

    const STATUSES = ['todo', 'doing', 'next-up', 'done'] as const;

    function isAppTaskStatus(value: string): value is AppTaskStatus {
        return STATUSES.some((status) => status === value);
    }

    function isTaskPriority(value: number): value is TaskPriority {
        return Number.isInteger(value) && value >= 0 && value <= 4;
    }

    function handleListChange(value: string): void {
        updateTask(task.id, (current) => ({ ...current, remoteCollectionId: value || undefined }));
    }

    function handleStatusChange(value: string): void {
        if (!isAppTaskStatus(value)) return;
        updateTask(task.id, (current) => ({ ...current, status: value }));
    }

    function handlePriorityChange(value: number): void {
        const clamped = Math.max(0, Math.min(4, value));
        if (!isTaskPriority(clamped)) return;
        updateTask(task.id, (current) => ({ ...current, priority: clamped }));
    }

    function handleDateChange(field: 'start' | 'due', value: string): void {
        updateTask(task.id, (current) => ({ ...current, [field]: withDate(current[field], value) }));
    }

    function clearDate(field: 'start' | 'due'): void {
        updateTask(task.id, (current) => ({ ...current, [field]: undefined }));
    }

    function handleTimeChange(field: 'start' | 'due', value: string): void {
        updateTask(task.id, (current) => ({ ...current, [field]: withTime(current[field], value) }));
    }

    function handleDurationChange(value: number): void {
        updateTask(task.id, (current) => ({ ...current, est: value || undefined }));
    }

    function handleParentChange(value: string): void {
        updateTask(task.id, (current) => ({ ...current, parentTask: value || undefined }));
    }

    function handleRecurrenceChange(value: string | undefined): void {
        updateTask(task.id, (current) => ({ ...current, rrule: value }));
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
            { label: 'Doing', value: 'doing' },
            { label: 'Next Up', value: 'next-up' },
            { label: 'Done', value: 'done' },
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
        value={task.priority ?? 2}
        onchange={handlePriorityChange}
        variant="bare"
        disabled={readOnly}
        ariaLabel="Priority"
    />
</InspectorField>

<InspectorField icon="list" label="List" disabled={readOnly}>
    <SelectInput
        value={task.remoteCollectionId ?? ''}
        onchange={handleListChange}
        options={listOptions}
        variant="bare"
        disabled={readOnly}
        ariaLabel="List"
    />
</InspectorField>

<InspectorField icon="calendar_month" label="Start" disabled={readOnly}>
    <div class="inspector-inline">
        <DateField
            value={getFormattedDate(task.start)}
            placeholder="No start date"
            ariaLabel="Start date"
            disabled={readOnly}
            onchange={(value) => handleDateChange('start', value)}
            onclear={() => clearDate('start')}
        />
        <TimeInput
            value={getFormattedTime(task.start)}
            onchange={(value) => handleTimeChange('start', value)}
            variant="bare"
            disabled={readOnly || !task.start}
            ariaLabel="Start time"
        />
    </div>
</InspectorField>

<InspectorField icon="event" label="Due" disabled={readOnly}>
    <div class="inspector-inline">
        <DateField
            value={getFormattedDate(task.due)}
            placeholder="No due date"
            ariaLabel="Due date"
            disabled={readOnly}
            onchange={(value) => handleDateChange('due', value)}
            onclear={() => clearDate('due')}
        />
        <TimeInput
            value={getFormattedTime(task.due)}
            onchange={(value) => handleTimeChange('due', value)}
            variant="bare"
            disabled={readOnly || !task.due}
            ariaLabel="Due time"
        />
    </div>
</InspectorField>

<InspectorField icon="timer" label="Estimate" disabled={readOnly}>
    <div class="inspector-inline">
        <NumberInput
            min={0}
            value={task.est ?? ''}
            onchange={handleDurationChange}
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
        onchange={(newTags: string[]) => {
            updateTask(task.id, (current) => ({
                ...current,
                tags: newTags.map((name: string) => ({ id: crypto.randomUUID(), name })),
            }));
        }}
        variant="bare"
        ariaLabel="Tags"
    />
</InspectorField>

<InspectorField icon="account_tree" label="Parent task" disabled={readOnly}>
    <SelectInput
        value={task.parentTask ?? ''}
        onchange={handleParentChange}
        options={parentOptions}
        variant="bare"
        disabled={readOnly}
        ariaLabel="Parent task"
    />
</InspectorField>

<InspectorField icon="repeat" label="Repeat" disabled={readOnly}>
    <RecurrenceEditor
        value={task.rrule}
        onchange={handleRecurrenceChange}
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

{#if task.completedAt}
    <InspectorField icon="task_alt" label="Completed">
        <span class="inspector-readonly-value">{formatTimestamp(task.completedAt)}</span>
    </InspectorField>
{/if}

<MetaFooter created={task.added} modified={task.updatedAt} onDelete={onDelete} deleteLabel="Delete task" deleteDisabled={readOnly} />
