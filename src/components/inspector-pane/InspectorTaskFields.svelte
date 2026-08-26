<script lang="ts">
    import type { AppTask, AppTaskStatus } from '../../lib/domain';
    import type { TaskPriority } from '../../lib/bindings/TaskPriority.generated';
    import { getFormattedDate } from './format';
    import SelectInput from '../inputs/SelectInput.svelte';
    import NumberInput from '../inputs/NumberInput.svelte';
    import TagsInput from '../inputs/TagsInput.svelte';
    import ChecklistInput from '../inputs/ChecklistInput.svelte';
    import type { ChecklistItem } from '../../lib/domain';
    import { store } from '../../lib/store.svelte';

    interface Props {
        task: AppTask;
        updateTask: (id: string, t: (t: AppTask) => AppTask) => void;
    }

    let { task, updateTask }: Props = $props();

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

    function handleDueChange(value: string): void {
        updateTask(task.id, (t) => ({ ...t, due: value || undefined }));
    }

    function handleDurationChange(val: number): void {
        updateTask(task.id, (t) => ({ ...t, est: val || undefined }));
    }

    function handleParentChange(value: string): void {
        updateTask(task.id, (t) => ({ ...t, parentTask: value || undefined }));
    }

    let parentOptions = $derived([
        { label: 'None', value: '' },
        ...store.tasks.filter(t => t.id !== task.id).map(t => ({ label: t.title, value: t.id }))
    ]);
</script>

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

<div class="inspector-field">
    <label for={`due-${task.id}`}>Due Date</label>
    <input
        id={`due-${task.id}`}
        class="inspector-date-input"
        type="date"
        value={getFormattedDate(task.due)}
        onchange={(e) => handleDueChange(e.currentTarget.value)}
    />
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

<div class="inspector-field">
    <label for={`checklist-${task.id}`}>Checklist</label>
    <ChecklistInput
        id={`checklist-${task.id}`}
        checklist={task.checklist ?? []}
        onchange={(newChecklist: ChecklistItem[]) => {
            updateTask(task.id, (t) => ({ ...t, checklist: newChecklist }));
        }}
    />
</div>
