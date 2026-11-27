import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
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
});
