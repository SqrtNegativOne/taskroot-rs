import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../test/tauriMock';
import type { AppCalendar, AppTaskColumnDef, AppTaskFilter, FilterGroup } from '../../lib/domain';
import { filterGroup } from '../../lib/domain';
import { defaultTaskListFilters, taskListState } from '../../lib/taskList.svelte';
import TaskListPane from './TaskListPane.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

const calendars: AppCalendar[] = [
    { id: 'cal-events', summary: 'Event Calendar', writable: true, component: 'VEVENT' },
    { id: 'cal-tasks', summary: 'Task List', writable: true, component: 'VTODO' },
];

const schema: AppTaskColumnDef[] = [
    { id: 'remotecollectionid', label: 'Collection', filter_type: 'Text', sortable: false },
];

describe('TaskListPane calendar filter', () => {
    beforeEach(() => {
        tauri.reset();
        taskListState.filters = defaultTaskListFilters();
        taskListState.sort = [];
        tauri.stubCommandValue('get_active_calendars', calendars);
        tauri.stubCommandValue('get_task_schema', schema);
        tauri.stubCommandValue('query_tasks', []);
        tauri.stubCommandValue('get_past_due_task_ids', []);
        tauri.stubCommandValue('get_ui_state', undefined);
    });

    afterEach(() => {
        cleanup();
    });

    it('offers only task lists as calendar filter values from the view menu', async () => {
        render(TaskListPane, { props: { onAddTask: vi.fn() } });

        await fireEvent.click(await screen.findByTitle('View'));
        await fireEvent.click(screen.getByRole('button', { name: /add filter/i }));
        await fireEvent.click(await screen.findByRole('button', { name: /select\.\.\./i }));

        expect(screen.getByRole('checkbox', { name: 'Task List' })).toBeTruthy();
        expect(screen.queryByRole('checkbox', { name: 'Event Calendar' })).toBeNull();
    });

    it('keeps filter and sort editors in the view menu and drops the lists manager', async () => {
        tauri.stubCommandValue('get_task_schema', [...schema, { id: 'due', label: 'Due', filter_type: 'Date', sortable: true }]);
        render(TaskListPane, { props: { onAddTask: vi.fn() } });

        expect(screen.queryByTitle('Manage lists')).toBeNull();
        expect(screen.queryByTitle('Filter')).toBeNull();
        expect(screen.queryByTitle('Sort')).toBeNull();

        await fireEvent.click(await screen.findByTitle('View'));
        expect(screen.getByRole('button', { name: /add filter/i })).toBeTruthy();
        expect(screen.getByRole('button', { name: /add sort/i })).toBeTruthy();
    });

    it('queries a nested definition and creates tasks with only unconditional typed defaults', async () => {
        const filters: FilterGroup<AppTaskFilter> = { operator: 'and', children: [
            { kind: 'condition', filter: { column: 'priority', operator: 'is', value: ['3'] } },
            { kind: 'condition', filter: { column: 'tags', operator: 'is', value: ['work'] } },
            { kind: 'group', ...filterGroup<AppTaskFilter>([
                { column: 'status', operator: 'is', value: ['todo'] },
                { column: 'status', operator: 'is', value: ['next-up'] },
            ], 'or') },
        ] };
        taskListState.filters = filters;
        const onAddTask = vi.fn();
        render(TaskListPane, { props: { onAddTask } });
        await waitFor(() => { expect(tauri.invoke).toHaveBeenCalledWith('query_tasks', { filters, sort: [], query: '' }); });
        await fireEvent.click(screen.getByTitle('Add Task'));
        expect(onAddTask).toHaveBeenCalledWith({ status: undefined, priority: 3, tags: [expect.objectContaining({ name: 'work' })] });
    });
});
