<script lang="ts">
    import type { AppTask, AppTaskStatus } from '../../lib/domain';
    import type { TaskPriority } from '../../lib/bindings/TaskPriority.generated';
    import { getFormattedDate } from './format';

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

    function handlePriorityChange(raw: string): void {
        const val = parseInt(raw) || 0;
        const clamped = Math.max(0, Math.min(4, val));
        if (!isTaskPriority(clamped)) return;
        updateTask(task.id, (t) => ({ ...t, priority: clamped }));
    }

    function handleDueChange(value: string): void {
        updateTask(task.id, (t) => ({ ...t, due: value || undefined }));
    }

    function handleDurationChange(value: string): void {
        updateTask(task.id, (t) => ({ ...t, est: value ? parseInt(value) : undefined }));
    }
</script>

<div class="inspector-row">
    <div class="inspector-field">
        <label for={`status-${task.id}`}>Status</label>
        <select
            id={`status-${task.id}`}
            value={task.status ?? 'todo'}
            onchange={(e) => handleStatusChange(e.currentTarget.value)}
        >
            <option value="todo">Todo</option>
            <option value="doing">Doing</option>
            <option value="next-up">Next Up</option>
            <option value="done">Done</option>
        </select>
    </div>
    <div class="inspector-field">
        <label for={`priority-${task.id}`}>Priority</label>
        <input
            id={`priority-${task.id}`}
            type="number"
            min="0"
            max="4"
            value={task.priority ?? 2}
            onchange={(e) => handlePriorityChange(e.currentTarget.value)}
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
    <input
        id={`duration-${task.id}`}
        type="number"
        placeholder="Unset"
        value={task.est ?? ''}
        onchange={(e) => handleDurationChange(e.currentTarget.value)}
    />
</div>
