<script lang="ts">
    import type { AppTask, AppTaskStatus, AppCalendar } from '../../lib/domain';
    import type { TaskPriority } from '../../lib/bindings/TaskPriority.generated';
    import { getFormattedDate, getFormattedTime, withDate, withTime } from './format';
    import SelectInput from '../inputs/SelectInput.svelte';
    import NumberInput from '../inputs/NumberInput.svelte';
    import TagsInput from '../inputs/TagsInput.svelte';
    import TimeInput from '../inputs/TimeInput.svelte';
    import { useAutoQuery } from '../../lib/safeInvoke.svelte';
    import { activeCalendars } from '../../lib/queries';

    interface Props {
        task: AppTask;
        tasks: AppTask[];
        updateTask: (id: string, t: (t: AppTask) => AppTask) => void;
    }

    let { task, tasks, updateTask }: Props = $props();

    const calendarsQuery = useAutoQuery<AppCalendar[]>('get_active_calendars', activeCalendars);

    let listOptions = $derived.by(() => {
        const todos = (calendarsQuery.data ?? []).filter(c => c.component === 'VTODO');
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

    function handleListChange(value: string): void {
        updateTask(task.id, (t) => ({ ...t, remoteCollectionId: value || undefined }));
    }

    function isTaskPriority(value: number): value is TaskPriority {
        return Number.isInteger(value) && value >= 0 && value <= 4;
    }

    const STATUSES = ['todo', 'doing', 'next-up', 'done'] as const;

    function isAppTaskStatus(value: string): value is AppTaskStatus {
        return STATUSES.some((status) => status === value);
    }

    function handleStatusChange(value: string): void {
        if (!isAppTaskStatus(value)) return;
        updateTask(task.id, (t) => ({ ...t, status: value }));
    }

    function handlePriorityChange(val: number): void {
        const clamped = Math.max(0, Math.min(4, val));
        if (!isTaskPriority(clamped)) return;
        updateTask(task.id, (t) => ({ ...t, priority: clamped }));
    }

    function handleDateChange(field: 'start' | 'due', value: string): void {
        updateTask(task.id, (t) => ({ ...t, [field]: withDate(t[field], value) }));
    }

    function handleTimeChange(field: 'start' | 'due', value: string): void {
        updateTask(task.id, (t) => ({ ...t, [field]: withTime(t[field], value) }));
    }

    function handleDurationChange(val: number): void {
        updateTask(task.id, (t) => ({ ...t, est: val || undefined }));
    }

    function handleParentChange(value: string): void {
        updateTask(task.id, (t) => ({ ...t, parentTask: value || undefined }));
    }

    let parentOptions = $derived([
        { label: 'None', value: '' },
        ...tasks.filter(t => t.id !== task.id).map(t => ({ label: t.title, value: t.id }))
    ]);
</script>

<div class="inspector-field">
    <label for={`list-${task.id}`}>List</label>
    <SelectInput
        value={task.remoteCollectionId ?? ''}
        onchange={handleListChange}
        options={listOptions}
    />
</div>

<div class="inspector-row">
    <div class="inspector-field">
        <label for={`status-${task.id}`}>Status</label>
        <SelectInput
            value={task.status ?? 'todo'}
            onchange={handleStatusChange}
            options={[
                { label: 'Todo', value: 'todo' },
                { label: 'Doing', value: 'doing' },
                { label: 'Next Up', value: 'next-up' },
                { label: 'Done', value: 'done' }
            ]}
        />
    </div>
    <div class="inspector-field">
        <label for={`priority-${task.id}`}>Priority</label>
        <NumberInput
            min={0}
            max={4}
            value={task.priority ?? 2}
            onchange={handlePriorityChange}
        />
    </div>
</div>

<div class="inspector-field-group">
    <label for={`start-${task.id}`}>Start</label>
    <div class="inspector-datetime">
        <input
            id={`start-${task.id}`}
            class="inspector-date-input"
            type="date"
            value={getFormattedDate(task.start)}
            onchange={(e) => handleDateChange('start', e.currentTarget.value)}
        />
        <TimeInput
            value={getFormattedTime(task.start)}
            onchange={(value: string) => handleTimeChange('start', value)}
        />
    </div>
</div>

<div class="inspector-field-group">
    <label for={`due-${task.id}`}>Due</label>
    <div class="inspector-datetime">
        <input
            id={`due-${task.id}`}
            class="inspector-date-input"
            type="date"
            value={getFormattedDate(task.due)}
            onchange={(e) => handleDateChange('due', e.currentTarget.value)}
        />
        <TimeInput
            value={getFormattedTime(task.due)}
            onchange={(value: string) => handleTimeChange('due', value)}
        />
    </div>
</div>

<div class="inspector-field">
    <label for={`duration-${task.id}`}>Duration (min)</label>
    <NumberInput
        value={task.est ?? ''}
        onchange={handleDurationChange}
    />
</div>

<div class="inspector-field">
    <label for={`tags-${task.id}`}>Tags</label>
    <TagsInput
        id={`tags-${task.id}`}
        tags={task.tags ? task.tags.map(t => t.name) : []}
        onchange={(newTags: string[]) => {
            updateTask(task.id, (t) => ({ ...t, tags: newTags.map((name: string) => ({ id: crypto.randomUUID(), name })) }));
        }}
    />
</div>

<div class="inspector-field">
    <label for={`parent-${task.id}`}>Parent Task</label>
    <SelectInput
        value={task.parentTask ?? ''}
        onchange={handleParentChange}
        options={parentOptions}
    />
</div>

