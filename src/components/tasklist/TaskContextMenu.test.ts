import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import type { AppTask } from '../../lib/domain';
import TaskContextMenu from './TaskContextMenu.svelte';

function makeTask(overrides: Partial<AppTask> = {}): AppTask {
    return { id: 'task-1', title: 'Task', ...overrides };
}

function renderMenu(task: AppTask) {
    const onClose = vi.fn();
    const onDelete = vi.fn();
    const onDo = vi.fn();
    render(TaskContextMenu, {
        props: { task, x: 40, y: 40, onClose, onDelete, onDo },
    });
    return { onClose, onDelete, onDo };
}

afterEach(() => {
    cleanup();
});

describe('TaskContextMenu', () => {
    it('starts the task and closes the menu', async () => {
        const task = makeTask();
        const { onClose, onDo } = renderMenu(task);

        await fireEvent.click(screen.getByRole('menuitem', { name: 'Do' }));

        expect(onDo).toHaveBeenCalledWith(task);
        expect(onClose).toHaveBeenCalled();
    });

    it('deletes the task and closes the menu', async () => {
        const task = makeTask();
        const { onClose, onDelete } = renderMenu(task);

        await fireEvent.click(screen.getByRole('menuitem', { name: 'Delete' }));

        expect(onDelete).toHaveBeenCalledWith(task.id);
        expect(onClose).toHaveBeenCalled();
    });
});
