import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import type { AppTask } from '../../lib/domain';
import { ymd } from '../../lib/time';
import TaskRow from './TaskRow.svelte';

function makeTask(overrides: Partial<AppTask> = {}): AppTask {
    return { id: 'task-1', title: 'Task', ...overrides };
}

function renderRow(task: AppTask) {
    return render(TaskRow, {
        props: { task, updateTask: vi.fn() },
    });
}

afterEach(() => {
    cleanup();
});

describe('TaskRow', () => {
    it('marks a recurring task with the recurrence icon', () => {
        renderRow(makeTask({ rrule: 'RRULE:FREQ=WEEKLY' }));

        expect(screen.getByTitle('Recurring task')).toBeTruthy();
    });

    it('marks a task with a future start date as scheduled', () => {
        renderRow(makeTask({ start: '2999-01-01' }));

        expect(screen.getByTitle('Scheduled start')).toBeTruthy();
        expect(screen.getByTitle('Scheduled start').classList.contains('is-today')).toBe(false);
        expect(screen.getByTitle('Scheduled start').classList.contains('is-past')).toBe(false);
    });

    it('marks a task starting today distinctly from a future start', () => {
        renderRow(makeTask({ start: ymd(new Date()) }));

        expect(screen.getByTitle('Starts today').classList.contains('is-today')).toBe(true);
    });

    it('marks a task whose start date has passed distinctly', () => {
        renderRow(makeTask({ start: '1999-01-01' }));

        expect(screen.getByTitle('Start date passed').classList.contains('is-past')).toBe(true);
    });

    it('leaves a task without a start date unmarked', () => {
        renderRow(makeTask());

        expect(screen.queryByTitle('Scheduled start')).toBeNull();
        expect(screen.queryByTitle('Starts today')).toBeNull();
        expect(screen.queryByTitle('Start date passed')).toBeNull();
    });

    it('leaves a non-recurring task unmarked', () => {
        renderRow(makeTask());

        expect(screen.queryByTitle('Recurring task')).toBeNull();
    });

    it('colours the circle by priority', () => {
        renderRow(makeTask({ priority: 2 }));

        expect(
            screen.getByRole('button', { name: 'Priority 2' }).classList.contains('pri-bg-2'),
        ).toBe(true);
    });

    it('offers the task context menu on right click', async () => {
        const task = makeTask();
        const onContextMenu = vi.fn();
        const { container } = render(TaskRow, {
            props: { task, updateTask: vi.fn(), onContextMenu },
        });

        await fireEvent.contextMenu(container.querySelector('.task-row')!);

        expect(onContextMenu).toHaveBeenCalledWith(expect.anything(), task);
    });
});
