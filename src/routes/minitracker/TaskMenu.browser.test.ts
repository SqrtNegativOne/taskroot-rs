import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import type { AppTask } from '../../lib/domain';
import TaskMenu from './TaskMenu.svelte';

const TASKS: AppTask[] = [
    { id: 't1', title: 'Review budget report', status: 'todo' },
    { id: 't2', title: 'Review design doc', status: 'todo' },
    { id: 't3', title: 'Review PR comments', status: 'todo' },
    { id: 't4', title: 'Buy groceries', status: 'todo' },
];

function renderMenu(tasks: AppTask[] = TASKS) {
    const onSelect = vi.fn();
    const onStop = vi.fn();
    const onSetCustom = vi.fn();
    const onToggleDone = vi.fn();
    const onAddTask = vi.fn();

    render(TaskMenu, {
        props: {
            tasks,
            onSelect,
            onStop,
            onSetCustom,
            onToggleDone,
            onAddTask,
        },
    });

    return { onSelect, onStop, onSetCustom, onToggleDone, onAddTask };
}

describe('TaskMenu Add task button', () => {
    afterEach(cleanup);

    it('is hidden when the search input is empty or whitespace', async () => {
        renderMenu();

        expect(screen.queryByRole('button', { name: /add task/i })).toBeNull();

        const input = screen.getByRole('searchbox');
        await fireEvent.input(input, { target: { value: '   ' } });

        expect(screen.queryByRole('button', { name: /add task/i })).toBeNull();
    });

    it('shows when 0 tasks match and calls onAddTask with the trimmed query', async () => {
        const { onAddTask } = renderMenu();
        const input = screen.getByRole('searchbox');

        await fireEvent.input(input, { target: { value: '  Write release notes  ' } });

        const addButton = screen.getByRole('button', { name: /add task/i });
        expect(addButton.textContent).toContain('Write release notes');

        await fireEvent.click(addButton);
        expect(onAddTask).toHaveBeenCalledTimes(1);
        expect(onAddTask).toHaveBeenCalledWith('Write release notes');
    });

    it('shows when 1 or 2 tasks match and calls onAddTask on click', async () => {
        const { onAddTask } = renderMenu();
        const input = screen.getByRole('searchbox');

        await fireEvent.input(input, { target: { value: 'groceries' } });
        expect(screen.getAllByRole('menuitem')).toHaveLength(1);
        const addOne = screen.getByRole('button', { name: /add task/i });

        await fireEvent.click(addOne);
        expect(onAddTask).toHaveBeenCalledWith('groceries');

        await fireEvent.input(input, { target: { value: 'Review b' } });
        expect(screen.getAllByRole('menuitem')).toHaveLength(1);
        expect(screen.getByRole('button', { name: /add task/i })).toBeTruthy();

        await fireEvent.input(input, { target: { value: 'd' } });
        expect(screen.getAllByRole('menuitem')).toHaveLength(2);
        expect(screen.getByRole('button', { name: /add task/i })).toBeTruthy();
    });

    it('is hidden when more than 2 tasks match', async () => {
        renderMenu();
        const input = screen.getByRole('searchbox');

        await fireEvent.input(input, { target: { value: 'Review' } });

        expect(screen.getAllByRole('menuitem')).toHaveLength(3);
        expect(screen.queryByRole('button', { name: /add task/i })).toBeNull();
    });
});
