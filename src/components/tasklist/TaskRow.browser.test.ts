import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import type { AppTask, TaskPatch } from '../../lib/domain';
import { ymd } from '../../lib/time';
import TaskRow from './TaskRow.svelte';

function makeTask(overrides: Partial<AppTask> = {}): AppTask {
    return { id: 'task-1', title: 'Task', ...overrides };
}

function renderRow(
    task: AppTask,
    updateTask = vi.fn<(id: string, patches: TaskPatch[]) => void>(),
) {
    return {
        ...render(TaskRow, {
            props: { task, updateTask },
        }),
        updateTask,
    };
}

describe('TaskRow', () => {
    it('distinguishes future, today, past, and unset start dates', () => {
        const future = renderRow(makeTask({ start: '2999-01-01' }));
        expect(screen.getByTitle('Scheduled start')).toBeTruthy();
        future.unmount();

        const today = renderRow(makeTask({ start: ymd(new Date()) }));
        expect(screen.getByTitle('Starts today')).toBeTruthy();
        today.unmount();

        const past = renderRow(makeTask({ start: '1999-01-01' }));
        expect(screen.getByTitle('Start date passed')).toBeTruthy();
        past.unmount();

        renderRow(makeTask());
        expect(screen.queryByTitle('Scheduled start')).toBeNull();
        expect(screen.queryByTitle('Starts today')).toBeNull();
        expect(screen.queryByTitle('Start date passed')).toBeNull();
    });

    it('marks only recurring tasks with the recurrence indicator', () => {
        const recurring = renderRow(makeTask({ rrule: 'RRULE:FREQ=WEEKLY' }));
        expect(screen.getByTitle('Recurring task')).toBeTruthy();
        recurring.unmount();

        renderRow(makeTask());
        expect(screen.queryByTitle('Recurring task')).toBeNull();
    });

    it('sends a status patch when toggling completion', async () => {
        const todo = renderRow(makeTask({ id: 'task-1', status: 'todo' }));
        const circle = todo.container.querySelector('.task-circle');
        expect(circle).not.toBeNull();
        await fireEvent.click(circle!);
        expect(todo.updateTask).toHaveBeenCalledWith('task-1', [{ field: 'status', value: 'done' }]);
        todo.unmount();

        const done = renderRow(makeTask({ id: 'task-2', status: 'done' }));
        const doneCircle = done.container.querySelector('.task-circle');
        await fireEvent.click(doneCircle!);
        expect(done.updateTask).toHaveBeenCalledWith('task-2', [{ field: 'status', value: 'todo' }]);
    });

    it('clears the checkmark when a task status stays or reverts to todo externally', async () => {
        const view = renderRow(makeTask({ id: 'task-3', status: 'todo' }));
        const circle = view.container.querySelector('.task-circle')!;
        await fireEvent.click(circle);

        await view.rerender({ task: makeTask({ id: 'task-3', status: 'done' }), updateTask: view.updateTask });
        expect(view.container.querySelector('.task-circle-check')).not.toBeNull();

        await view.rerender({ task: makeTask({ id: 'task-3', status: 'todo' }), updateTask: view.updateTask });
        expect(view.container.querySelector('.task-circle-check')).toBeNull();
    });
});
